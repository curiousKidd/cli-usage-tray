use crate::{tray, usage::{self, store::OffsetStore, Accumulator, LocalUsage, Roots, Tool, ToolRoots}};
use chrono::{DateTime, Utc};
use std::{path::PathBuf, process::Command, sync::{atomic::{AtomicBool, Ordering}, Mutex}, time::Instant};
use tauri::{AppHandle, Emitter, Manager, State};

pub struct UsageState {
    pub accumulator: Mutex<Accumulator>,
    pub roots: Mutex<Roots>,
    pub offsets: Mutex<OffsetStore>,
    pub last_scan_at: Mutex<Option<DateTime<Utc>>>,
    pub warnings: Mutex<Vec<String>>,
    pub scanning: Mutex<bool>,
    pub dialog_open: AtomicBool,
    pub last_blur_hide: Mutex<Option<Instant>>,
}

impl UsageState {
    pub fn load() -> Self {
        Self {
            accumulator: Mutex::new(Accumulator::load()),
            roots: Mutex::new(Roots::load()),
            offsets: Mutex::new(OffsetStore::in_default_dir().unwrap_or_else(|_| OffsetStore::new(PathBuf::from(".digimon/ingest")))),
            last_scan_at: Mutex::new(None), warnings: Mutex::new(Vec::new()), scanning: Mutex::new(false),
            dialog_open: AtomicBool::new(false), last_blur_hide: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn local_usage(range: String, state: State<'_, UsageState>) -> Result<LocalUsage, String> {
    valid_range(&range)?;
    let accumulator = state.accumulator.lock().map_err(|_| "usage store unavailable")?;
    let last_scan_at = *state.last_scan_at.lock().map_err(|_| "usage state unavailable")?;
    let warnings = state.warnings.lock().map_err(|_| "usage state unavailable")?.clone();
    let mut usage = usage::aggregate(&accumulator, &range, last_scan_at, warnings);
    usage.scanning = *state.scanning.lock().map_err(|_| "usage state unavailable")?;
    Ok(usage)
}

#[tauri::command]
pub fn rescan(app: AppHandle, state: State<'_, UsageState>) -> Result<LocalUsage, String> {
    rescan_state(&app, &state)
}

pub fn rescan_app(app: &AppHandle) -> Result<LocalUsage, String> {
    let state = app.state::<UsageState>();
    rescan_state(app, &state)
}

fn rescan_state(app: &AppHandle, state: &State<'_, UsageState>) -> Result<LocalUsage, String> {
    *state.scanning.lock().map_err(|_| "usage state unavailable")? = true;
    let warnings = {
        let roots = state.roots.lock().map_err(|_| "usage roots unavailable")?;
        let mut offsets = state.offsets.lock().map_err(|_| "usage offsets unavailable")?;
        let mut accumulator = state.accumulator.lock().map_err(|_| "usage store unavailable")?;
        usage::scan_all(&roots, &mut offsets, &mut accumulator)
    };
    let now = Utc::now();
    *state.last_scan_at.lock().map_err(|_| "usage state unavailable")? = Some(now);
    *state.warnings.lock().map_err(|_| "usage state unavailable")? = warnings;
    *state.scanning.lock().map_err(|_| "usage state unavailable")? = false;
    let usage = local_usage("today".into(), state.clone())?;
    tray::update_title(app, usage.total.total_tokens);
    let _ = app.emit("usage://updated", ());
    Ok(usage)
}

#[tauri::command]
pub fn usage_roots(state: State<'_, UsageState>) -> Result<Vec<ToolRoots>, String> {
    Ok(state.roots.lock().map_err(|_| "usage roots unavailable")?.to_tool_roots())
}

#[tauri::command]
pub fn set_usage_roots(tool: Tool, paths: Vec<String>, state: State<'_, UsageState>) -> Result<Vec<ToolRoots>, String> {
    let mut roots = state.roots.lock().map_err(|_| "usage roots unavailable")?;
    roots.set_custom(tool, paths);
    roots.save().map_err(|error| error.to_string())?;
    Ok(roots.to_tool_roots())
}

#[tauri::command]
pub fn reset_usage_roots(state: State<'_, UsageState>) -> Result<Vec<ToolRoots>, String> {
    let mut roots = state.roots.lock().map_err(|_| "usage roots unavailable")?;
    roots.reset_custom();
    roots.save().map_err(|error| error.to_string())?;
    Ok(roots.to_tool_roots())
}

#[tauri::command]
pub fn float_show(app: AppHandle) -> Result<(), String> { show_float(&app) }

pub fn show_float(app: &AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("float").ok_or("float window unavailable")?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn float_hide(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("float").ok_or("float window unavailable")?.hide().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn begin_dialog(state: State<'_, UsageState>) { state.dialog_open.store(true, Ordering::Relaxed); }

#[tauri::command]
pub fn end_dialog(state: State<'_, UsageState>) { state.dialog_open.store(false, Ordering::Relaxed); }

#[tauri::command]
pub fn close_window(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("float").ok_or("float window unavailable")?.close().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_scale(scale: f64, app: AppHandle) -> Result<(), String> {
    if !scale_options().contains(&scale) { return Err("unsupported scale".into()); }
    app.get_webview_window("float").ok_or("float window unavailable")?.set_zoom(scale).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn scale_options() -> Vec<f64> { vec![0.8, 0.9, 1.0, 1.1, 1.25] }

#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let path = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
        .join("Library/Logs/com.digimon.adventure-clone");
    Command::new("open").arg(path).spawn().map(|_| ()).map_err(|error| error.to_string())
}

fn valid_range(range: &str) -> Result<(), String> {
    if matches!(range, "today" | "week" | "all") { Ok(()) } else { Err("range must be today, week, or all".into()) }
}
