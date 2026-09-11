// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod importers;
pub mod model;
pub mod normalization;
pub mod pack;
pub mod search;

use model::{EntryRecord, SearchSuggestion};
use search::{PackInfo, PackLoadError, SearchEngine};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub engine: Mutex<Option<SearchEngine>>,
    pub diagnostics: Mutex<PackDiagnostics>,
}

/// Where packs were looked for and what happened. The status bar is derived from this
/// instead of the hardcoded "Ready. Local dictionary packs loaded." it used to show —
/// which claimed success even when zero packs had loaded.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PackDiagnostics {
    pub pack_dir: String,
    pub loaded: usize,
    pub errors: Vec<PackLoadError>,
}

/// Candidate pack directories, most specific first.
///
/// `PathBuf::from("packs")` alone — the previous behaviour — is relative to the process
/// working directory. Installed via MSI and launched from the Start menu, the app found
/// no packs and reported success. Resource dir first so a bundled pack wins, then the
/// per-user data dir for packs the user installs, then the CWD for `cargo run`.
fn pack_dir_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(resource) = app.path().resource_dir() {
        dirs.push(resource.join("packs"));
    }
    if let Ok(data) = app.path().app_data_dir() {
        dirs.push(data.join("packs"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("packs"));
        }
    }
    dirs.push(PathBuf::from("packs"));
    dirs
}

#[tauri::command]
fn suggest(query: String, state: State<'_, AppState>) -> Result<Vec<SearchSuggestion>, String> {
    let guard = state.engine.lock().map_err(|e| e.to_string())?;
    if let Some(engine) = guard.as_ref() {
        engine.suggest(&query, 10).map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn get_entry(entry_id: String, state: State<'_, AppState>) -> Result<Option<EntryRecord>, String> {
    let guard = state.engine.lock().map_err(|e| e.to_string())?;
    if let Some(engine) = guard.as_ref() {
        engine.get_entry(&entry_id).map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn list_packs(state: State<'_, AppState>) -> Result<Vec<PackInfo>, String> {
    let guard = state.engine.lock().map_err(|e| e.to_string())?;
    if let Some(engine) = guard.as_ref() {
        Ok(engine.list_packs())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn toggle_pack(pack_id: String, enabled: bool, state: State<'_, AppState>) -> Result<bool, String> {
    let mut guard = state.engine.lock().map_err(|e| e.to_string())?;
    if let Some(engine) = guard.as_mut() {
        Ok(engine.set_pack_enabled(&pack_id, enabled))
    } else {
        Ok(false)
    }
}

#[tauri::command]
fn pack_diagnostics(state: State<'_, AppState>) -> Result<PackDiagnostics, String> {
    let guard = state.diagnostics.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            engine: Mutex::new(None),
            diagnostics: Mutex::new(PackDiagnostics::default()),
        })
        .setup(|app| {
            let mut engine = SearchEngine::new();
            let mut diagnostics = PackDiagnostics::default();

            // Load from every candidate directory, not just the first that works: the
            // installer bundles only the small fixture pack, while real packs live in the
            // per-user data directory. add_pack skips paths already loaded.
            let candidates = pack_dir_candidates(app.handle());
            let mut searched = Vec::new();
            for dir in &candidates {
                searched.push(dir.to_string_lossy().to_string());
                if dir.is_dir() {
                    diagnostics.loaded += engine.load_directory(dir).unwrap_or(0);
                }
            }
            diagnostics.pack_dir = searched.join(", ");
            // Only report failures for directories that exist; a missing candidate is
            // expected, not an error worth showing the user.
            diagnostics.errors = engine
                .load_errors()
                .iter()
                .filter(|e| e.message != "pack directory not found")
                .cloned()
                .collect();

            let state: State<'_, AppState> = app.state();
            *state.engine.lock().unwrap() = Some(engine);
            *state.diagnostics.lock().unwrap() = diagnostics;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            suggest,
            get_entry,
            list_packs,
            toggle_pack,
            pack_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
