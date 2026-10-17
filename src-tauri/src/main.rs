// Hides the console on Windows in release builds (no-op on Linux).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Wayland: the activation token KDE gives the process launched by the shortcut must be
    // passed as an argument, so the already running instance (which only receives the arguments) can use it.
    let token = std::env::var("XDG_ACTIVATION_TOKEN").or_else(|_| std::env::var("DESKTOP_STARTUP_ID")).ok().filter(|t| !t.is_empty());
    if let Some(t) = token {
        if !std::env::args().any(|a| a.starts_with("--activation-token=")) {
            use std::os::unix::process::CommandExt;
            if let Ok(exe) = std::env::current_exe() {
                let err = std::process::Command::new(exe).args(std::env::args().skip(1)).arg(format!("--activation-token={t}")).exec();
                eprintln!("restart with activation token failed: {err}");
            }
        }
    }
    if std::env::args().any(|a| a == "--version") {
        println!("{}", gestione_attivita_lib::version());
        return;
    }
    for (flag, install) in [("--update", true), ("--check-update", false)] {
        if std::env::args().any(|a| a == flag) {
            std::process::exit(gestione_attivita_lib::update_cli(install));
        }
    }
    if std::env::args().any(|a| a == "--dump") {
        return gestione_attivita_lib::dump();
    }
    for (flag, on) in [("--apply-shortcuts", true), ("--remove-shortcuts", false)] {
        if std::env::args().any(|a| a == flag) {
            std::process::exit(if gestione_attivita_lib::shortcuts(on) { 0 } else { 1 });
        }
    }
    gestione_attivita_lib::run()
}
