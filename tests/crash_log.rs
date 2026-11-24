#[path = "../src/crash.rs"]
mod crash;

#[test]
fn a_panic_writes_a_report() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "rustium-crash-test-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("RUSTIUM_LOG_DIR", &dir);
    std::env::set_var("RUSTIUM_CRASH_SILENT", "1");

    crash::install();
    let _ = std::panic::catch_unwind(|| panic!("crash-log-test"));

    let log = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|e| e.to_str()) == Some("log"))
        .expect("a crash log was written");
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(text.contains("Rustium crash report"));
    assert!(text.contains("crash-log-test"));
    assert!(text.contains("component:"));

    let _ = std::fs::remove_dir_all(&dir);
}
