// Nasconde la console su Windows in release (no-op su Linux).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Wayland: il permesso di attivazione che KDE dà al processo lanciato dalla scorciatoia va
    // passato come argomento, così l'istanza già aperta (che riceve solo gli argomenti) può usarlo.
    let token = std::env::var("XDG_ACTIVATION_TOKEN").or_else(|_| std::env::var("DESKTOP_STARTUP_ID")).ok().filter(|t| !t.is_empty());
    if let Some(t) = token {
        if !std::env::args().any(|a| a.starts_with("--activation-token=")) {
            use std::os::unix::process::CommandExt;
            if let Ok(exe) = std::env::current_exe() {
                let err = std::process::Command::new(exe).args(std::env::args().skip(1)).arg(format!("--activation-token={t}")).exec();
                eprintln!("riavvio con il permesso di attivazione non riuscito: {err}");
            }
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
