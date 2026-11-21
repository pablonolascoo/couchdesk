use tauri::Manager;

mod apps;
mod backdrop;
mod browser;
mod claude;
pub mod deskzones;
mod emulators;
pub mod mcp;
mod files;
mod games;
mod hardware;
mod hotkeys;
mod i18n;
mod idle;
mod input;
mod localsend;
pub mod media;
pub mod memory;
pub mod multitask;
mod pads;
mod padpoll;
pub mod paths;
pub mod perf;
pub mod psmando;
mod receive;
mod remote;
mod settings;
mod transfer;
pub mod startup;
mod scr;
mod voice;
mod weather;
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
  // Salvapantallas de Windows: /s enseñar, /c ajustes, /p miniatura (no la hacemos)
  let mode = scr::parse_args(&std::env::args().collect::<Vec<_>>());
  if mode == scr::LaunchMode::Preview {
    return;
  }
  scr::set_saver_only(mode == scr::LaunchMode::Saver);
  if mode == scr::LaunchMode::Config {
    scr::set_launch_route(scr::SAVER_ROUTE);
  }
  let mut context = tauri::generate_context!();
  if scr::saver_only() {
    // Solo el salvapantallas: la ventana principal ni se ve
    context.config_mut().app.windows.iter_mut().for_each(|w| w.visible = false);
  }
  tauri::Builder::default()
    // Debe ir primero: una segunda consola se cierra al instante y despierta a la primera
    .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
      use tauri::Emitter;
      match scr::parse_args(&args) {
        // Windows pide el salvapantallas y la consola ya está abierta: se enseña el suyo
        scr::LaunchMode::Saver => {
          let app = app.clone();
          std::thread::spawn(move || idle::show(&app, false));
        }
        scr::LaunchMode::Preview => {}
        scr::LaunchMode::Config => {
          scr::set_saver_only(false);
          browser::go_home(app);
          let _ = app.emit("claude-navigate", scr::SAVER_ROUTE);
        }
        // Abrirla otra vez (menú Inicio, botón PS…) = volver al Inicio de la que ya está
        scr::LaunchMode::Normal => {
          scr::set_saver_only(false);
          browser::go_home(app);
        }
      }
    }))
    .manage(games::GameCache::default())
    .manage(games::Launchables::default())
    .manage(receive::ReceiveState::default())
    .manage(claude::ClaudeState::default())
    // Mando sin retraso: padpoll.rs solo lee con la ventana principal delante
    .on_window_event(|w, e| {
      if let tauri::WindowEvent::Focused(on) = e {
        if w.label() == "main" {
          padpoll::set_focused(*on);
        }
      }
    })
    .setup(|app| {
      perf::mark_setup();
      // Antes «Consola»: sus datos pasan a %LOCALAPPDATA%\CouchDesk (una sola vez, sin copiar)
      paths::migrate_legacy();
      // Abierta por Windows solo como salvapantallas: nada más que eso
      if scr::saver_only() {
        let handle = app.handle().clone();
        idle::start(handle.clone());
        std::thread::spawn(move || {
          std::thread::sleep(std::time::Duration::from_millis(150));
          idle::show(&handle, false);
          if let Some(main) = handle.get_webview_window("main") {
            let _ = main.close();
          }
        });
        return Ok(());
      }
      // Mando sin retraso (hilo propio con prioridad de juego) y plan de energía al jugar
      if let Some(main) = app.get_webview_window("main") {
        padpoll::set_focused(main.is_focused().unwrap_or(true));
      }
      padpoll::start(app.handle().clone());
      perf::start();
      // Si el arranque de Windows apunta a una consola vieja (actualizada o movida), se arregla.
      // En otro hilo: nada lento antes del primer pintado.
      std::thread::spawn(|| {
        startup::heal_shell();
        scr::refresh_copy();
      });
      // Mando móvil siempre disponible (solo red local + código secreto del QR)
      let hub = remote::Hub::new(Some(app.handle().clone()));
      app.manage(hub.clone());
      // LocalSend: la consola aparece en la app LocalSend del móvil o de otro PC
      let ls = localsend::LocalSend::new(Some(app.handle().clone()));
      app.manage(ls.clone());
      tauri::async_runtime::spawn(localsend::serve(ls));
      // Salidas de emergencia: Esc mantenido / 3 veces y el botón Xbox del mando
      hotkeys::start(app.handle().clone());
      // Multitarea (botón Xbox/PS corto, Ctrl+Alt+Inicio) y gestor de memoria
      multitask::start(app.handle().clone());
      memory::start(app.handle().clone());
      // Salvapantallas: vigila cuánto tiempo lleva el PC sin tocarse
      idle::start(app.handle().clone());
      // Claude: recoge los pedidos de su servidor MCP (abrir pantallas, webs, Inicio)
      claude::start(app.handle().clone());
      // Escritorio organizado: zonas en el escritorio de Windows cuando está a la vista
      deskzones::start(app.handle().clone());
      // Actualizar emuladores instalados en segundo plano, un rato después de arrancar
      // Si la consola es el escritorio, arrancar los programas de inicio (RustDesk, WARP, audio…)
      std::thread::spawn(|| {
        if !startup::explorer_running() {
          startup::run_startup_items();
        }
        // Mandos de PlayStation: lo mismo que xTend hace con los de Xbox, más el touchpad
        psmando::spawn_helper();
      });
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
      i18n::set_locale,
      media::media_now_playing,
      media::media_control,
      media::media_volume_get,
      media::media_volume_set,
      media::media_open_spotify,
      pads::pads_list,
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
      localsend::localsend_status,
      localsend::localsend_destinations,
      localsend::localsend_set_dest,
      localsend::localsend_answer,
      remote::remote_status,
      remote::remote_mode,
      webs::site_previews,
      backdrop::backdrop_list_media,
      backdrop::backdrop_import,
      hardware::get_hardware,
      emulators::list_emulators,
      emulators::install_emulator,
      emulators::rollback_emulator,
      emulators::launch_emulator,
      claude::claude_available,
      claude::claude_ask,
      claude::claude_reset,
      claude::claude_status,
      claude::claude_conversations,
      claude::claude_conversation,
      claude::claude_conversation_delete,
      claude::claude_connectors,
      claude::claude_connector_add,
      claude::claude_connector_toggle,
      claude::claude_connector_remove,
      claude::claude_plugins,
      claude::claude_plugin_install,
      claude::claude_plugin_remove,
      claude::assistant_toggle,
      claude::assistant_show,
      claude::assistant_hide,
      voice::voice_start,
      voice::voice_stop,
      voice::voice_open_settings,
      browser::browser_open,
      browser::browser_in_system,
      browser::cloud_open,
      emulators::open_remote_play,
      emulators::emu_folder,
      startup::shell_status,
      startup::shell_set,
      scr::launch_route_take,
      scr::windows_saver_status,
      scr::windows_saver_set,
      scr::open_windows_settings,
      scr::run_optimizer,
      idle::screensaver_settings_get,
      idle::screensaver_settings_set,
      idle::screensaver_show,
      idle::screensaver_preview,
      idle::screensaver_hide,
      idle::screensaver_status,
      idle::screensaver_inhibit,
      weather::weather_search,
      weather::weather_now,
      weather::weather_locate,
      deskzones::deskzones_get,
      deskzones::deskzones_set,
      deskzones::deskzones_status,
      deskzones::deskzones_organize,
      deskzones::deskzones_restore,
      deskzones::deskzones_frames,
      deskzones::deskzones_preview,
      perf::perf_app_ready,
      perf::perf_status,
      perf::perf_tweak_set,
      perf::perf_undo_all,
      perf::perf_game_plan_set,
      perf::perf_open_settings,
      multitask::multitask_list,
      multitask::multitask_visible,
      multitask::multitask_switch,
      multitask::multitask_back,
      multitask::multitask_close,
      multitask::multitask_alive,
      multitask::multitask_force_close,
      memory::memory_status,
      memory::memory_settings_set,
      memory::memory_free
    ])
    .build(context)
    .expect("error while building tauri application")
    .run(|app, event| {
      // Al cerrar: deshacer prioridades y modo eficiencia puestos por el gestor de memoria
      if let tauri::RunEvent::Exit = event {
        memory::restore_all(app);
      }
    });
}
