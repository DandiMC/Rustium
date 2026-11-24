//! One crash report per panic, written where the launcher's own folder lives.
//!
//! The game executables run without a visible terminal most of the time, so a
//! panic used to disappear. Every binary installs this at the top of `main()`;
//! a panic leaves a text file with the build, the machine and the backtrace,
//! and points the player at it.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Installs the hook once. Calling it again is harmless.
pub fn install() {
    if INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let path = write_log(info);
        if std::env::var_os("RUSTIUM_CRASH_SILENT").is_none() {
            if let Some(path) = path.as_deref() {
                show_dialog(path);
            }
        }
        previous(info);
    }));
}

fn log_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("RUSTIUM_LOG_DIR") {
        return PathBuf::from(dir);
    }
    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("Rustium").join("logs");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("Rustium");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
            return PathBuf::from(state).join("Rustium").join("logs");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("Rustium")
                .join("logs");
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("logs")
}

fn write_log(info: &std::panic::PanicHookInfo<'_>) -> Option<PathBuf> {
    let dir = log_dir();
    fs::create_dir_all(&dir).ok()?;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("crash-{secs}-{}.log", std::process::id()));

    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(|s| s.as_str()))
        .unwrap_or("(unknown panic payload)");

    let mut text = String::new();
    let _ = writeln!(text, "Rustium crash report");
    let _ = writeln!(text, "version: {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(
        text,
        "component: {}",
        option_env!("CARGO_BIN_NAME").unwrap_or("rustium")
    );
    let _ = writeln!(text, "os: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    let _ = writeln!(text, "process id: {}", std::process::id());
    if let Ok(exe) = std::env::current_exe() {
        let _ = writeln!(text, "executable: {}", exe.display());
    }
    if let Ok(cwd) = std::env::current_dir() {
        let _ = writeln!(text, "working directory: {}", cwd.display());
    }
    let _ = writeln!(text, "args: {:?}", std::env::args().collect::<Vec<_>>());
    for key in [
        "RUSTIUM_GAME_VERSION",
        "RUSTIUM_LANG",
        "RD_BENCH",
        "RD_WIDTH",
        "RD_HEIGHT",
    ] {
        if let Ok(value) = std::env::var(key) {
            let _ = writeln!(text, "{key}: {value}");
        }
    }

    let _ = writeln!(text);
    let _ = writeln!(text, "panic: {message}");
    if let Some(location) = info.location() {
        let _ = writeln!(text, "location: {location}");
    }
    let thread = std::thread::current();
    let _ = writeln!(
        text,
        "thread: {:?} ({:?})",
        thread.id(),
        thread.name().unwrap_or("unnamed")
    );
    let _ = writeln!(
        text,
        "\nReport it at https://github.com/DandiMC/Rustium/issues and attach this file."
    );
    let _ = writeln!(text, "\nbacktrace:\n{}", std::backtrace::Backtrace::force_capture());

    fs::write(&path, text.as_bytes()).ok()?;
    // A stable name makes it easy to find even when the timestamped files pile up.
    let _ = fs::write(dir.join("last-crash.txt"), path.to_string_lossy().as_bytes());
    Some(path)
}

#[cfg(windows)]
fn show_dialog(path: &Path) {
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut c_void,
            text: *const u16,
            caption: *const u16,
            utype: u32,
        ) -> i32;
    }

    let body = format!(
        "Rustium se cerró por un error.\n\nEl registro está en:\n{}\n\nRustium crashed.\n\nThe log is at:\n{}\n\nhttps://github.com/DandiMC/Rustium/issues",
        path.display(),
        path.display()
    );
    let text: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();
    let caption: Vec<u16> = "Rustium".encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            0x0000_0010,
        );
    }
}

#[cfg(not(windows))]
fn show_dialog(path: &Path) {
    eprintln!("Rustium crashed. Log: {}", path.display());
}
