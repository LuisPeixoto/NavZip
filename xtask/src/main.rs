mod chromium;
mod icon;
mod monitor;
mod native_capture;
mod native_pipeline;
mod release;
use anyhow::{Result, bail};
use std::{path::PathBuf, process::Command};

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    std::env::set_current_dir(&root)?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let engine = chromium::Engine::new(root)?;
    match args.first().map(String::as_str).unwrap_or("help") {
        "doctor" => engine.doctor()?,
        "icons" => icon::generate(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap(),
        )?,
        "fetch" | "sync" => engine.sync()?,
        "apply" => engine.apply()?,
        "build" => engine.build(args.iter().any(|a| a == "--release"))?,
        "build-integration" => engine.build_integration()?,
        "run" => engine.launch(args.get(1..).unwrap_or_default())?,
        "check" => engine.check()?,
        "monitor" => engine.monitor()?,
        "capture-window" if args.len() == 3 => {
            engine.capture_window(args[1].parse()?, &PathBuf::from(&args[2]))?
        }
        "wait-native" if args.len() == 2 => engine.wait_native(args[1].parse()?)?,
        "package" => engine.package()?,
        "prepare-release" if args.len() == 2 => {
            release::prepare(&std::env::current_dir()?, &args[1])?
        }
        "test" => chromium::run(Command::new("cargo").args(["test", "--workspace", "--locked"]))?,
        "overlay-check" => engine.overlay_check(args.get(1).map(PathBuf::from))?,
        "export-profile" if args.len() == 3 => chromium::run(Command::new("cargo").args([
            "run",
            "--locked",
            "-p",
            "navzip-helper",
            "--",
            "export-profile",
            &args[1],
            &args[2],
        ]))?,
        "help" => println!(
            "NavZip / Chromium 155\n  cargo xtask doctor\n  cargo xtask fetch\n  cargo xtask apply\n  cargo xtask build [--release]\n  cargo xtask build-integration\n  cargo xtask run -- [argumentos Chromium]\n  cargo xtask check\n  cargo xtask monitor\n  cargo xtask capture-window PID SAIDA.png\n  cargo xtask wait-native PID_DO_BUILD\n  cargo xtask package\n  cargo xtask prepare-release TAG (source only)\n  cargo xtask test\n  cargo xtask overlay-check [arvore-fonte]\n  cargo xtask export-profile PERFIL_LEGADO DIRETORIO_NOVO\n\nNAVZIP_CHROMIUM_ROOT seleciona o diretório que contém src/. Não há fallback Servo, Electron ou CEF."
        ),
        other => bail!("Comando desconhecido: {other}; use cargo xtask help"),
    }
    Ok(())
}
