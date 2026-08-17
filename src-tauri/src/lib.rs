mod commands;
mod docking;
mod dock_manager;
mod fs;
mod recent;
mod settings;
mod terminal;
mod terminal_provider;

use commands::*;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_project,
            pick_folder,
            list_dir,
            read_file,
            save_file,
            rename_path,
            create_file,
            create_dir,
            delete_path,
            copy_path,
            get_file_tree,
            create_terminal,
            close_terminal,
            write_terminal,
            resize_terminal,
            get_git_status,
            list_terminal_providers,
            check_accessibility_permission,
            request_accessibility_permission,
            get_terminal_settings,
            save_terminal_settings,
            start_terminal,
            set_dock_slot,
            redock_terminal,
            stop_terminal,
            get_dock_session,
            save_session,
            load_session,
            recent::add_recent_project,
            recent::get_recent_projects,
        ])
        .on_menu_event(|app, event| recent::on_menu_event(app, event.id().as_ref()))
        .setup(|app| {
            // 最近打开记录 + 系统菜单 + Dock 菜单
            recent::init(app.handle());
            if let Err(e) = recent::build_menu(app.handle()) {
                eprintln!("build menu failed: {e}");
            }
            #[cfg(target_os = "macos")]
            recent::dock::install();
            if cfg!(debug_assertions) {
                if let Err(e) = app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                ) {
                    eprintln!("log plugin init failed: {e}");
                }
            }
            // 加载设置：app_config_dir 失败不允许中断 setup，
            // 否则 Tauri 会把错误带进 did_finish_launching（无法 unwind）直接 abort
            let config_dir = match app.path().app_config_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("app_config_dir failed, using temp fallback: {e}");
                    std::env::temp_dir().join("modou-fallback-config")
                }
            };
            if let Err(e) = std::fs::create_dir_all(&config_dir) {
                eprintln!("create config dir failed: {e}");
            }
            let settings_path = settings::TerminalSettings::path_for(&config_dir);
            let terminal_settings = settings::TerminalSettings::load(&settings_path);

            // 初始化 DockManager
            if let Some(window) = app.get_webview_window("main") {
                let dock_manager = dock_manager::DockManager::new(app.handle().clone(), window.clone());
                let state = app.state::<AppState>();
                *state.dock_manager.lock().unwrap() = Some(dock_manager);
                *state.settings.lock().unwrap() = terminal_settings;
            }

            Ok(())
        })
       .on_window_event(|window, event| {
           if matches!(
               event,
               tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
           ) {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                let guard = state.dock_manager.lock().unwrap();
                if let Some(ref dock) = guard.as_ref() {
                   dock.send(dock_manager::DockCmd::Redock);
               }
           }
           if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                let guard = state.dock_manager.lock().unwrap();
                if let Some(ref dock) = guard.as_ref() {
                   dock.send(dock_manager::DockCmd::Shutdown);
               }
           }
       })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
