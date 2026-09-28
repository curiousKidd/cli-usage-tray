mod usage;
mod commands;
mod tray;

use tauri::Manager;
use std::{sync::atomic::Ordering, time::Instant};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::UsageState::load())
        .setup(|app| {
            tray::init(app.handle())?;
            let window = app.get_webview_window("float").unwrap();
            let float_window = window.clone();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| match event {
                tauri::WindowEvent::Focused(false) => {
                    let state = app_handle.state::<commands::UsageState>();
                    if !state.dialog_open.load(Ordering::Relaxed) && float_window.hide().is_ok() {
                        if let Ok(mut last_blur_hide) = state.last_blur_hide.lock() { *last_blur_hide = Some(Instant::now()); }
                    }
                }
                _ => {}
            });
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let _ = commands::rescan_app(&handle);
                std::thread::sleep(std::time::Duration::from_secs(60));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::local_usage, commands::rescan, commands::usage_roots,
            commands::set_usage_roots, commands::reset_usage_roots,
            commands::float_show, commands::float_hide, commands::close_window,
            commands::set_scale, commands::scale_options, commands::open_log_dir,
            commands::begin_dialog, commands::end_dialog,
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
