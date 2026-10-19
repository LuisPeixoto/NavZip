//! A one-shot continuation of an already running local build, not a scheduler.

use anyhow::{Result, bail};
use std::path::Path;

#[cfg(target_os = "macos")]
pub fn wait_and_test(pid: u32, root: &Path, binary: &Path) -> Result<()> {
    use anyhow::Context;
    use std::{
        fs,
        io::{Read, Seek, SeekFrom},
        process::Command,
    };

    let pid: i32 = pid.try_into().context("PID inválido")?;
    if pid <= 0 {
        bail!("Informe o PID positivo do build NavZip atual");
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(".cache/chromium-pipeline.lock"))?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .context("Já existe uma continuação de build neste workspace")?;
    let args = Command::new("/bin/ps")
        .args(["-ww", "-p", &pid.to_string(), "-o", "command="])
        .output()?;
    let args = String::from_utf8(args.stdout)?;
    let args = args.trim();
    let absolute = root.join("target/debug/xtask");
    if !args.ends_with("xtask build")
        || !(args.starts_with("target/debug/xtask ")
            || args.starts_with(&format!("{} ", absolute.display())))
    {
        bail!("O PID informado não corresponde ao build NavZip esperado");
    }
    let cwd = Command::new("/usr/sbin/lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()?;
    let cwd = String::from_utf8(cwd.stdout)?;
    let directory = cwd.lines().find_map(|line| line.strip_prefix('n'));
    if directory
        .map(Path::new)
        .map(Path::canonicalize)
        .transpose()?
        != Some(root.canonicalize()?)
    {
        bail!("O build observado não pertence a este workspace");
    }
    let exit = BuildExit::observe(pid)?;
    let status = root.join(".cache/chromium-pipeline.json");
    let write_status = |state: &str, error: Option<String>| -> Result<()> {
        let value = serde_json::json!({
            "state": state, "buildPid": pid, "nativeBinary": binary,
            "error": error, "releaseValidated": false,
            "updatedAtUnix": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?.as_secs(),
            "buildProgress": build_progress(&root.join(".cache/chromium-native-build.log")),
        });
        let temporary = status.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&value)?)?;
        fs::rename(temporary, &status)?;
        Ok(())
    };
    write_status("waiting_for_build", None)?;
    println!("Aguardando o build NavZip PID {pid}; o teste nativo só inicia após sucesso.");
    let result = (|| -> Result<()> {
        exit.wait(|| write_status("waiting_for_build", None))?;
        let mut log = fs::File::open(root.join(".cache/chromium-native-build.log"))?;
        let length = log.metadata()?.len().min(16 * 1024);
        log.seek(SeekFrom::End(-(length as i64)))?;
        let mut tail = Vec::new();
        log.read_to_end(&mut tail)?;
        if !String::from_utf8_lossy(&tail).contains("Build Succeeded:") || !binary.is_file() {
            bail!("O log e o executável não confirmam um build Chromium concluído");
        }
        write_status("running_native_smoke", None)?;
        crate::chromium::run(
            Command::new("npm")
                .current_dir(root)
                .args(["run", "test:native"]),
        )?;
        write_status("native_smoke_passed", None)?;
        println!(
            "Teste nativo aprovado. Comparação visual e homologação de release continuam pendentes."
        );
        Ok(())
    })();
    if let Err(error) = &result {
        write_status("failed", Some(format!("{error:#}")))?;
    }
    result
}

pub(crate) fn build_progress(path: &Path) -> Option<serde_json::Value> {
    use std::io::{Read, Seek, SeekFrom};
    let mut log = std::fs::File::open(path).ok()?;
    let length = log.metadata().ok()?.len().min(16 * 1024);
    log.seek(SeekFrom::End(-(length as i64))).ok()?;
    let mut tail = Vec::new();
    log.read_to_end(&mut tail).ok()?;
    for line in String::from_utf8_lossy(&tail).lines().rev() {
        let Some((counts, _)) = line.strip_prefix('[').and_then(|s| s.split_once(']')) else {
            continue;
        };
        let Some((completed, total)) = counts.split_once('/') else {
            continue;
        };
        if let (Ok(completed), Ok(total)) = (completed.parse::<u64>(), total.parse::<u64>()) {
            return Some(serde_json::json!({
                "completedSteps": completed, "totalSteps": total,
                "lastBuildMessage": line,
                "note": "O total pode mudar durante a avaliação do grafo; não é uma estimativa de tempo.",
            }));
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn wait_and_test(_pid: u32, _root: &Path, _binary: &Path) -> Result<()> {
    bail!("A continuação de um build já iniciado está disponível somente no macOS");
}

#[cfg(target_os = "macos")]
struct BuildExit(std::os::fd::OwnedFd);

#[cfg(target_os = "macos")]
impl BuildExit {
    fn observe(pid: i32) -> Result<Self> {
        use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
        // SAFETY: kqueue returns a new descriptor owned by this invocation.
        let descriptor = unsafe { libc::kqueue() };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: descriptor is valid and has not been transferred or closed.
        let queue = unsafe { OwnedFd::from_raw_fd(descriptor) };
        let change = libc::kevent {
            ident: pid as usize,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ONESHOT,
            fflags: libc::NOTE_EXIT | libc::NOTE_EXITSTATUS,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        // SAFETY: change is a valid record, and no output records are requested.
        let count = unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                &change,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        if count < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(Self(queue))
    }

    fn wait(self, mut progress: impl FnMut() -> Result<()>) -> Result<()> {
        use std::os::fd::AsRawFd;
        let mut event = libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        let count = loop {
            let timeout = libc::timespec {
                tv_sec: 10,
                tv_nsec: 0,
            };
            // SAFETY: event is writable for one record. The registered process
            // identity remains attached to this queue even if its PID is reused.
            let count = unsafe {
                libc::kevent(
                    self.0.as_raw_fd(),
                    std::ptr::null(),
                    0,
                    &mut event,
                    1,
                    &timeout,
                )
            };
            if count == 0 {
                progress()?;
                continue;
            }
            if count > 0 {
                break count;
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error.into());
            }
        };
        let data = event.data;
        if count != 1 || event.flags & libc::EV_ERROR != 0 || event.fflags & libc::NOTE_EXIT == 0 {
            bail!("Não foi possível observar o encerramento do build: {data}");
        }
        if data != 0 {
            bail!("O build não terminou com sucesso (status do processo: {data})");
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn proceeds_only_after_a_successful_process_exit() -> Result<()> {
        let mut child = Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?;
        let exit = BuildExit::observe(child.id() as i32)?;
        drop(child.stdin.take());
        let observed = exit.wait(|| Ok(()));
        assert!(child.wait()?.success());
        observed?;
        Ok(())
    }

    #[test]
    fn rejects_an_interrupted_build() -> Result<()> {
        let mut child = Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?;
        let exit = BuildExit::observe(child.id() as i32)?;
        child.kill()?;
        let observed = exit.wait(|| Ok(()));
        assert!(!child.wait()?.success());
        assert!(observed.is_err());
        Ok(())
    }
}
