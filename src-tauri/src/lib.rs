use tauri::Manager;

mod apps;
mod browser;
mod claude;
mod emulators;
pub mod mcp;
mod files;
mod games;
mod hardware;
mod hotkeys;
mod input;
pub mod psmando;
mod receive;
mod remote;
mod settings;
pub mod startup;
mod webs;

/// Cierra la consola y deja ver el escritorio normal de Windows
/// (si la consola hacía de escritorio, arranca primero el de Windows).
/// `normal = true`: además Windows vuelve a arrancar en su escritorio de siempre.
#[tauri::command(async)]
fn exit_to_desktop(app: tauri::AppHandle, normal: Option<bool>) -> Result<(), String> {
  if normal.unwrap_or(false) {
    startup::set_shell(false)?;
  }
  startup::open_windows_desktop();
  app.exit(0);
  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    // Debe ir primero: una segunda consola se cierra al instante y despierta a la primera
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      // Abrirla otra vez (menú Inicio, botón PS…) = volver al Inicio de la que ya está
      browser::go_home(app);
    }))
    .manage(games::GameCache::default())
    .manage(games::Launchables::default())
    .manage(receive::ReceiveState::default())
    .manage(claude::ClaudeState::default())
    .setup(|app| {
      // Mando móvil siempre disponible (solo red local + código secreto del QR)
      let hub = remote::Hub::new(Some(app.handle().clone()));
      app.manage(hub.clone());
      // Salidas de emergencia: Esc mantenido / 3 veces y el botón Xbox del mando
      hotkeys::start(app.handle().clone());
      // Actualizar emuladores instalados en segundo plano, un rato después de arrancar
      // Si la consola es el escritorio, arrancar los programas de inicio (RustDesk, WARP, audio…)
      if !startup::explorer_running() {
        startup::run_startup_items();
      }
      // Mandos de PlayStation: lo mismo que xTend hace con los de Xbox, más el touchpad
      psmando::spawn_helper();
      std::thread::spawn(|| {
        // xTend: mando como ratón y teclado en todo Windows (START + BACK cambia de modo)
        if let Err(e) = emulators::ensure_xtend_running() {
          log::warn!("xTend: {e}");
        }
        std::thread::sleep(std::time::Duration::from_secs(30));
        emulators::update_all_installed();
      });
      tauri::async_runtime::spawn(async move {
        if let Err(e) = remote::serve(hub, remote::PORT).await {
          log::error!("Mando móvil: {e}");
        }
      });
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      exit_to_desktop,
      games::list_games,
      games::launch_game,
      games::list_everything,
      games::mark_as_game,
      settings::get_volume,
      settings::set_volume,
      settings::set_mute,
      settings::get_brightness,
      settings::set_brightness,
      settings::get_battery,
      settings::get_power_plans,
      settings::set_power_plan,
      settings::power_action,
      settings::get_wifi,
      settings::connect_wifi,
      settings::disconnect_wifi,
      files::list_places,
      files::list_dir,
      files::open_path,
      files::paste_items,
      files::new_folder,
      files::trash_items,
      receive::receive_status,
      receive::receive_start,
      receive::receive_stop,
      remote::remote_status,
      remote::remote_mode,
      webs::site_previews,
      hardware::get_hardware,
      emulators::list_emulators,
      emulators::install_emulator,
      emulators::rollback_emulator,
      emulators::launch_emulator,
      claude::claude_available,
      claude::claude_ask,
      claude::claude_reset,
      browser::browser_open,
      browser::browser_in_system,
      browser::cloud_open,
      emulators::open_remote_play,
      emulators::emu_folder,
      startup::shell_status,
      startup::shell_set
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
