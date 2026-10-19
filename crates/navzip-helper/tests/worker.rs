use anyhow::Result;
use navzip_extensions::runtime::{Action, run_worker};

#[test]
fn binary_accepts_the_isolated_worker_protocol() -> Result<()> {
    let module = wat::parse_str(
        r#"(module
      (import "navzip" "emit" (func $emit (param i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "{\22action\22:\22close_active_tab\22}")
      (func (export "run") (result i32)
        i32.const 0 i32.const 29 call $emit drop i32.const 0))"#,
    )?;
    let actions = run_worker(
        std::path::Path::new(env!("CARGO_BIN_EXE_navzip-helper")),
        &module,
    )?;
    assert!(matches!(actions.as_slice(), [Action::CloseActiveTab]));
    Ok(())
}

#[test]
fn binary_rejects_an_infinite_guest_without_hanging() -> Result<()> {
    let module = wat::parse_str(
        "(module (func (export \"run\") (result i32) (loop $forever br $forever) i32.const 0))",
    )?;
    let started = std::time::Instant::now();
    assert!(
        run_worker(
            std::path::Path::new(env!("CARGO_BIN_EXE_navzip-helper")),
            &module
        )
        .is_err()
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(35));
    Ok(())
}
