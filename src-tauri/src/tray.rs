use tauri::{image::Image, menu::{Menu, MenuItem, PredefinedMenuItem}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, AppHandle, Manager};
use std::time::Duration;

const TRAY_ID: &str = "usage";

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&refresh, &settings, &separator, &quit])?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(placeholder_icon())
        .icon_as_template(true)
        .title("0")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "refresh" => { let _ = crate::commands::rescan_app(app); },
            "settings" => { let _ = crate::commands::show_float(app); },
            "quit" => app.exit(0),
            _ => {},
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }) {
                let state = tray.app_handle().state::<crate::commands::UsageState>();
                if state.last_blur_hide.lock().ok().is_some_and(|last_hide| last_hide.as_ref().is_some_and(|last_hide| last_hide.elapsed() < Duration::from_millis(300))) { return; }
                if let Some(window) = tray.app_handle().get_webview_window("float") {
                    if window.is_visible().unwrap_or(false) { let _ = window.hide(); }
                    else { let _ = crate::commands::show_float(tray.app_handle()); }
                }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn update_title(app: &AppHandle, total: u64) {
    if let Some(icon) = app.tray_by_id(TRAY_ID) { let _ = icon.set_title(Some(&abbreviated(total))); }
}

fn abbreviated(tokens: u64) -> String {
    if tokens < 1_000 { tokens.to_string() }
    else if tokens < 1_000_000 { format!("{:.1}K", tokens as f64 / 1_000.0) }
    else { format!("{:.1}M", tokens as f64 / 1_000_000.0) }
}

fn placeholder_icon() -> Image<'static> {
    let mut pixels = vec![0; 16 * 16 * 4];
    for y in 0..16 {
        for x in 0..16 {
            let dx = 2 * x as i32 - 15;
            let dy = 2 * y as i32 - 15;
            if dx * dx + dy * dy <= 100 {
                pixels[(y * 16 + x) * 4 + 3] = 255;
            }
        }
    }
    Image::new_owned(pixels, 16, 16)
}
