mod migration;

use anyhow::{Result, bail};
use std::io::Read;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("export-profile") if args.len() == 3 => {
            let output = migration::export(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
            )?;
            println!("{}", output.display());
        }
        Some("--extension-worker") if args.len() == 1 => navzip_extensions::runtime::worker()?,
        Some("validate-migration") => {
            let mut input = String::new();
            std::io::stdin()
                .take(32 * 1024 * 1024 + 1)
                .read_to_string(&mut input)?;
            if input.len() > 32 * 1024 * 1024 {
                bail!("Migração excede 32 MiB");
            }
            migration::validate(&serde_json::from_str(&input)?)?;
        }
        _ => bail!(
            "navzip-helper export-profile PERFIL DIRETORIO_NOVO | validate-migration | --extension-worker (Wasm pela entrada padrão)"
        ),
    }
    Ok(())
}
