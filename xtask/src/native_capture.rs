//! Captures only a window owned by the freshly built NavZip executable.
//! Screenshot permissions remain under macOS control; no desktop fallback.

#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::{Result, bail};
use std::path::Path;

#[cfg(target_os = "macos")]
pub fn capture(pid: u32, executable: &Path, output: &Path) -> Result<()> {
    use std::{ffi::c_void, os::unix::ffi::OsStrExt, process::Command};

    type CfRef = *const c_void;
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGWindowListCopyWindowInfo(options: u32, relative_to: u32) -> CfRef;
        static kCGWindowOwnerPID: CfRef;
        static kCGWindowLayer: CfRef;
        static kCGWindowNumber: CfRef;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFArrayGetCount(array: CfRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CfRef, index: isize) -> CfRef;
        fn CFDictionaryGetValueIfPresent(dict: CfRef, key: CfRef, value: *mut CfRef) -> u8;
        fn CFNumberGetValue(number: CfRef, number_type: isize, output: *mut c_void) -> u8;
        fn CFRelease(value: CfRef);
    }
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_pidpath(pid: i32, buffer: *mut c_void, size: u32) -> i32;
    }

    struct WindowInfo(CfRef);
    impl Drop for WindowInfo {
        fn drop(&mut self) {
            // SAFETY: this non-null reference is owned by the Copy call below.
            unsafe { CFRelease(self.0) };
        }
    }
    fn number(dict: CfRef, key: CfRef) -> Option<i32> {
        let mut value = std::ptr::null();
        let mut result = 0i32;
        // SAFETY: dictionaries and the documented CFNumber keys are borrowed
        // from a retained CGWindowListCopyWindowInfo result. Type 3 is SInt32.
        unsafe {
            if CFDictionaryGetValueIfPresent(dict, key, &mut value) != 0
                && !value.is_null()
                && CFNumberGetValue(value, 3, (&mut result as *mut i32).cast()) != 0
            {
                Some(result)
            } else {
                None
            }
        }
    }

    let pid: i32 = pid.try_into().context("PID fora do intervalo permitido")?;
    if pid <= 0 {
        bail!("Informe o PID positivo do NavZip recém-compilado");
    }
    let executable = executable
        .canonicalize()
        .context("Compile o NavZip primeiro")?;
    let verify_process = || -> Result<()> {
        let mut buffer = [0u8; 4096];
        // SAFETY: buffer is writable for all 4096 bytes; pid is a positive i32.
        let size = unsafe { proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
        if size <= 0 {
            bail!("Processo NavZip indisponível: PID {pid}");
        }
        let end = buffer
            .iter()
            .position(|b| *b == 0)
            .context("Caminho inválido")?;
        let actual = Path::new(std::ffi::OsStr::from_bytes(&buffer[..end])).canonicalize()?;
        if actual != executable {
            bail!("Captura recusada: o PID não pertence ao executável NavZip deste checkout");
        }
        Ok(())
    };
    verify_process()?;
    // SAFETY: 1 = on-screen windows; 16 = exclude desktop elements. The returned
    // array is retained until WindowInfo drops, including all borrowed values.
    let windows = unsafe { CGWindowListCopyWindowInfo(1 | 16, 0) };
    if windows.is_null() {
        bail!("macOS não disponibilizou a lista de janelas para captura");
    }
    let windows = WindowInfo(windows);
    let mut window_id = None;
    // SAFETY: all indexes stay within the retained CFArray's count.
    unsafe {
        for i in 0..CFArrayGetCount(windows.0) {
            let dict = CFArrayGetValueAtIndex(windows.0, i);
            if number(dict, kCGWindowOwnerPID) == Some(pid)
                && number(dict, kCGWindowLayer) == Some(0)
            {
                window_id = number(dict, kCGWindowNumber);
                if window_id.is_some_and(|id| id > 0) {
                    break;
                }
            }
        }
    }
    let window_id = window_id
        .filter(|id| *id > 0)
        .context("Janela NavZip visível não encontrada")?;
    verify_process()?;
    if output
        .extension()
        .is_none_or(|extension| extension != "png")
    {
        bail!("A captura deve usar um arquivo .png");
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Reserve a new filename; do not overwrite a user-provided file or symlink.
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    let result = (|| -> Result<()> {
        let status = Command::new("/usr/sbin/screencapture")
            .args(["-x", "-o", "-t", "png", &format!("-l{window_id}")])
            .arg(output)
            .status()?;
        if !status.success() {
            bail!("Captura da janela recusada pelo macOS ({status}); não foi capturado o desktop");
        }
        let (width, height) = image::image_dimensions(output)?;
        if width == 0 || height == 0 {
            bail!("A captura retornou uma imagem vazia");
        }
        println!(
            "Janela nativa NavZip: {width} × {height}, {}",
            output.display()
        );
        Ok(())
    })();
    if result.is_err() {
        // This is exclusively the file just reserved by this invocation.
        let _ = std::fs::remove_file(output);
    }
    result
}

#[cfg(not(target_os = "macos"))]
pub fn capture(_pid: u32, _executable: &Path, _output: &Path) -> Result<()> {
    bail!("A captura de moldura nativa ainda está disponível somente no macOS");
}
