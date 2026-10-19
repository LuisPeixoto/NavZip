use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use wasmi::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    CreateTab { url: String },
    CloseActiveTab,
    SetTheme { accent: [u8; 3], dark: bool },
    Store { key: String, value: String },
}
impl Action {
    pub fn permission(&self) -> &'static str {
        match self {
            Self::CreateTab { .. } => "tabs.create",
            Self::CloseActiveTab => "tabs.close",
            Self::SetTheme { .. } => "theme",
            Self::Store { .. } => "storage",
        }
    }
}
struct State {
    limits: StoreLimits,
    actions: Vec<Action>,
}
/// No WASI, filesystem, network, clocks, credentials or arbitrary host imports.
pub fn execute(bytes: &[u8]) -> Result<Vec<Action>> {
    if bytes.len() > 8 * 1024 * 1024 {
        bail!("Módulo muito grande");
    }
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, bytes)?;
    let mut store = Store::new(
        &engine,
        State {
            limits: StoreLimitsBuilder::new()
                .memory_size(16 * 1024 * 1024)
                .memories(1)
                .tables(2)
                .instances(1)
                .trap_on_grow_failure(true)
                .build(),
            actions: vec![],
        },
    );
    store.limiter(|s| &mut s.limits);
    store.set_fuel(1_000_000)?;
    let mut linker = Linker::new(&engine);
    linker.func_wrap(
        "navzip",
        "emit",
        |mut caller: Caller<'_, State>, ptr: i32, len: i32| -> i32 {
            if ptr < 0 || !(1..=65536).contains(&len) || caller.data().actions.len() >= 32 {
                return -1;
            }
            let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
                return -1;
            };
            let mut buffer = vec![0; len as usize];
            if memory.read(&caller, ptr as usize, &mut buffer).is_err() {
                return -1;
            }
            let Ok(action) = serde_json::from_slice::<Action>(&buffer) else {
                return -1;
            };
            caller.data_mut().actions.push(action);
            0
        },
    )?;
    let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;
    let run = instance.get_typed_func::<(), i32>(&store, "run")?;
    if run.call(&mut store, ())? != 0 {
        bail!("A extensão retornou um erro");
    }
    Ok(store.into_data().actions)
}
/// The host must validate every action again against the installed manifest.
pub fn authorize(
    manifest: &crate::package::Manifest,
    action: &Action,
    private: bool,
) -> Result<()> {
    manifest.validate()?;
    if private {
        bail!("Extensões desativadas em janelas privadas");
    }
    if !manifest.permits(action.permission()) {
        bail!("A extensão não tem permissão para esta ação");
    }
    match action {
        Action::CreateTab { url } => {
            let u = url::Url::parse(url)?;
            if !matches!(u.scheme(), "https" | "http")
                || u.host_str().is_none()
                || u.password().is_some()
                || !u.username().is_empty()
            {
                bail!("Endereço não permitido");
            }
        }
        Action::Store { key, value }
            if key.is_empty() || key.len() > 128 || value.len() > 65536 =>
        {
            bail!("Armazenamento excede os limites")
        }
        _ => {}
    }
    Ok(())
}
pub fn worker() -> Result<()> {
    use std::io::{Read, Write};
    // Readiness precedes guest input. Startup latency is not Wasm execution.
    std::io::stdout().write_all(b"NAVZIP-WORKER/1 READY\n")?;
    std::io::stdout().flush()?;
    let mut bytes = vec![];
    std::io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    let actions = execute(&bytes)?;
    serde_json::to_writer(std::io::stdout(), &actions)?;
    Ok(())
}
pub fn run_worker(executable: &std::path::Path, bytes: &[u8]) -> Result<Vec<Action>> {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    if bytes.len() > 8 * 1024 * 1024 {
        bail!("Módulo muito grande");
    }
    let mut child = Command::new(executable)
        .arg("--extension-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut input = child
        .stdin
        .take()
        .context("Canal de extensão indisponível")?;
    let owned = bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = input.write_all(&owned);
    });
    let output = child
        .stdout
        .take()
        .context("Canal de extensão indisponível")?;
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut output = BufReader::new(output);
        let mut header = vec![];
        output.by_ref().take(64).read_until(b'\n', &mut header)?;
        if header != b"NAVZIP-WORKER/1 READY\n" {
            let _ = ready_tx.send(false);
            return Err(std::io::Error::other("Protocolo do worker inválido"));
        }
        let _ = ready_tx.send(true);
        let mut bytes = vec![];
        output
            .by_ref()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let start = Instant::now();
    let mut execution_started = None;
    loop {
        if execution_started.is_none() && matches!(ready_rx.try_recv(), Ok(true)) {
            execution_started = Some(Instant::now());
        }
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                let _ = writer.join();
                let _ = reader.join();
                bail!("A extensão foi interrompida");
            }
            break;
        }
        let timed_out = execution_started.map_or_else(
            || start.elapsed() > Duration::from_secs(30),
            |execution: Instant| execution.elapsed() > Duration::from_secs(3),
        );
        if timed_out {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writer.join();
            let _ = reader.join();
            bail!("A extensão excedeu o tempo permitido");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = writer.join();
    let output = reader
        .join()
        .map_err(|_| anyhow::anyhow!("Falha na extensão"))??;
    if output.len() > 2 * 1024 * 1024 {
        bail!("Resposta da extensão excedeu o limite");
    }
    Ok(serde_json::from_slice(&output)?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_checks_permission_and_private_policy_again() {
        let manifest: crate::package::Manifest = serde_json::from_str(include_str!(
            "../../../examples/extensions/ocean-theme/manifest.json"
        ))
        .unwrap();
        assert!(authorize(&manifest, &Action::CloseActiveTab, false).is_err());
        assert!(
            authorize(
                &manifest,
                &Action::SetTheme {
                    accent: [1, 2, 3],
                    dark: true
                },
                true
            )
            .is_err()
        );
        assert!(
            authorize(
                &manifest,
                &Action::SetTheme {
                    accent: [1, 2, 3],
                    dark: true
                },
                false
            )
            .is_ok()
        );
    }
    #[test]
    fn shipped_command_example_emits_valid_message() {
        let wasm = wat::parse_str(include_str!(
            "../../../examples/extensions/open-rust/command.wat"
        ))
        .unwrap();
        let actions = execute(&wasm).unwrap();
        assert!(matches!(&actions[0],Action::CreateTab{url}if url=="https://www.rust-lang.org/"));
    }
    #[test]
    fn infinite_execution_is_stopped() {
        let wasm =
            wat::parse_str("(module (func (export \"run\") (result i32) (loop br 0) i32.const 0))")
                .unwrap();
        assert!(execute(&wasm).is_err());
    }
    #[test]
    fn memory_growth_and_foreign_imports_are_denied() {
        let large = wat::parse_str(
            "(module (memory 1024) (func (export \"run\") (result i32) i32.const 0))",
        )
        .unwrap();
        assert!(execute(&large).is_err());
        let wasi=wat::parse_str("(module (import \"wasi_snapshot_preview1\" \"fd_write\" (func)) (func (export \"run\") (result i32) i32.const 0))").unwrap();
        assert!(execute(&wasi).is_err());
    }
    #[test]
    fn emits_only_structured_actions() {
        let json = r#"{"action":"close_active_tab"}"#;
        let escaped = json.replace('"', "\\\"");
        let wasm=wat::parse_str(format!("(module (import \"navzip\" \"emit\" (func $emit (param i32 i32) (result i32))) (memory (export \"memory\") 1) (data (i32.const 0) \"{escaped}\") (func (export \"run\") (result i32) i32.const 0 i32.const {} call $emit))",json.len())).unwrap();
        assert!(matches!(
            &execute(&wasm).unwrap()[0],
            Action::CloseActiveTab
        ));
    }
}
