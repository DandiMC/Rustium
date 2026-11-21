//! Regression test for the legacy renderer.
//!
//! The builds with zombies used to abort as soon as a lit one was drawn before
//! the fog layer: the fog pass kept the model pipeline bound, and wgpu refused
//! the draw. `RD_SMOKE` puts a zombie three blocks in front of the camera, draws
//! a few frames and closes the game; the test fails if the process aborts, if
//! the window never reaches the fog pass, or if no zombie was drawn.
//!
//! rd-132211 has no mobs and no model pipeline, so it cannot take this path.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run_smoke(name: &str, exe: &str) {
    let dir = std::env::temp_dir().join(format!("rustium-smoke-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("no se pudo crear la carpeta temporal");
    let log_path = dir.join("smoke.log");
    let out = std::fs::File::create(&log_path).expect("no se pudo crear el log");
    let err = out.try_clone().expect("no se pudo duplicar el log");

    let mut child = Command::new(exe)
        .env("RD_SMOKE", "1")
        .env("RD_SEED_FIJO", "1")
        .env("RD_FIJA", "1")
        .env("RD_POS", "128.5,128.5,43.0")
        .env("RD_ROT", "0,0")
        .current_dir(&dir)
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .unwrap_or_else(|e| panic!("{name}: no arranca: {e}"));

    let deadline = Instant::now() + Duration::from_secs(25);
    let status = loop {
        if let Some(status) = child.try_wait().expect("no se pudo esperar al juego") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            let txt = std::fs::read_to_string(&log_path).unwrap_or_default();
            panic!("{name}: no terminó a tiempo\n{txt}");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let txt = std::fs::read_to_string(&log_path).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(status.success(), "{name}: salió con {status}\n{txt}");
    assert!(
        !txt.contains("panicked") && !txt.contains("wgpu error"),
        "{name}: el juego se cerró solo\n{txt}"
    );

    let smoke = txt
        .lines()
        .find(|l| l.starts_with("SMOKE OK "))
        .unwrap_or_else(|| panic!("{name}: no llegó a dibujar\n{txt}"));
    let count = |key: &str| -> usize {
        smoke
            .split(key)
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };
    assert!(count("lit=") > 0, "{name}: no dibujó el zombi\n{txt}");
    assert!(count("fog=") > 0, "{name}: no dibujó la niebla\n{txt}");
}

#[test]
fn cada_version_con_zombis_abre_con_uno_delante() {
    let builds = [
        ("rd-132328", env!("CARGO_BIN_EXE_rd132328")),
        ("rd-20090515", env!("CARGO_BIN_EXE_rd20090515")),
        ("rd-160052", env!("CARGO_BIN_EXE_rd160052")),
        ("rd-161348", env!("CARGO_BIN_EXE_rd161348")),
    ];
    for (name, exe) in builds {
        run_smoke(name, exe);
    }
}
