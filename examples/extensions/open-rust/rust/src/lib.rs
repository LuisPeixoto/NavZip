#![no_std]

#[link(wasm_import_module = "navzip")]
unsafe extern "C" {
    fn emit(pointer: *const u8, length: usize) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn run() -> i32 {
    const ACTION: &[u8] = br#"{"action":"create_tab","url":"https://www.rust-lang.org/"}"#;
    // The immutable buffer stays valid throughout this synchronous host call.
    unsafe { emit(ACTION.as_ptr(), ACTION.len()) }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
