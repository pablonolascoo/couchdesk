// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  // Para medir cuánto tarda en verse la primera pantalla (perf.rs)
  app_lib::perf::mark_start();
  // Claude Code arranca la consola en este modo para usar sus herramientas
  if std::env::args().any(|a| a == "--mcp") {
    app_lib::mcp::serve_stdio();
    return;
  }
  // Proceso de fondo: mandos de PS4 / PS5 como ratón y teclado, touchpad como ratón
  if std::env::args().any(|a| a == "--mando-ps") {
    app_lib::psmando::run_helper();
    return;
  }
  // Vuelta atrás de emergencia: arranque normal de Windows otra vez
  if std::env::args().any(|a| a == "--restaurar-escritorio") {
    let _ = app_lib::startup::set_shell(false);
    app_lib::startup::open_windows_desktop();
    return;
  }
  app_lib::run();
}
