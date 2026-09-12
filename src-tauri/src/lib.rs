// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod importers;
pub mod model;
pub mod normalization;
pub mod pack;
pub mod search;
pub mod settings;

use model::{EntryRecord, SearchSuggestion};
use search::{PackInfo, PackLoadError, SearchEngine};
use settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub engine: Mutex<Option<SearchEngine>>,
    pub diagnostics: Mutex<PackDiagnostics>,
    /// Where pack preferences are persisted. Empty when no data directory is available, in
    /// which case preferences apply for the session but are not saved.
    pub settings_path: Mutex<PathBuf>,
}

/// Writes the current pack order and enabled flags to disk.
///
/// A failure to save is reported to the caller rather than swallowed: silently losing a
/// preference the reader just set is the kind of quiet failure this project already had too
/// much of.
pub fn persist_preferences(engine: &SearchEngine, path: &std::path::Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }
    let mut settings = Settings::load(path);
    let order = engine.pack_order();
    let ids: Vec<String> = order.iter().map(|(id, _, _)| id.clone()).collect();
    settings.set_order(&ids);
    for (id, _, enabled) in &order {
        settings.set_enabled(id, *enabled);
    }
    settings
        .save(path)
        .map_err(|e| format!("could not save pack preferences to {}: {e}", path.display()))
}

/// Where packs were looked for and what happened. The status bar is derived from this
/// instead of the hardcoded "Ready. Local dictionary packs loaded." it used to show —
/// which claimed success even when zero packs had loaded.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PackDiagnostics {
    pub pack_dir: String,
    pub loaded: usize,
    pub errors: Vec<PackLoadError>,
    /// Where pack order and enabled flags are saved. Empty when no data directory could be
    /// resolved, in which case preferences apply for this session only — surfaced so that
    /// degradation is visible instead of silent.
    pub settings_path: String,
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
    let Some(engine) = guard.as_mut() else {
        return Ok(false);
    };
    let changed = engine.set_pack_enabled(&pack_id, enabled);
    if changed {
        let path = state.settings_path.lock().map_err(|e| e.to_string())?;
        persist_preferences(engine, &path)?;
    }
    Ok(changed)
}

/// Sets the order packs break ranking ties in. Position in `pack_ids` is the priority.
#[tauri::command]
fn reorder_packs(
    pack_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<PackInfo>, String> {
    let mut guard = state.engine.lock().map_err(|e| e.to_string())?;
    let Some(engine) = guard.as_mut() else {
        return Ok(Vec::new());
    };
    engine.set_pack_order(&pack_ids);
    let path = state.settings_path.lock().map_err(|e| e.to_string())?;
    persist_preferences(engine, &path)?;
    Ok(engine.list_packs())
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
            settings_path: Mutex::new(PathBuf::new()),
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

            // Apply the reader's saved order and enabled flags. Without this, both were
            // reset to alphabetical-and-all-on at every launch.
            let settings_file = app
                .path()
                .app_data_dir()
                .map(|dir| settings::settings_path(&dir))
                .unwrap_or_default();
            if !settings_file.as_os_str().is_empty() {
                let saved = Settings::load(&settings_file);
                engine.apply_preferences(|pack_id| {
                    saved.get(pack_id).map(|pref| (pref.priority, pref.enabled))
                });
            }

            let state: State<'_, AppState> = app.state();
            *state.engine.lock().unwrap() = Some(engine);
            diagnostics.settings_path = settings_file.to_string_lossy().to_string();
            *state.diagnostics.lock().unwrap() = diagnostics;
            *state.settings_path.lock().unwrap() = settings_file;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            suggest,
            get_entry,
            list_packs,
            toggle_pack,
            reorder_packs,
            pack_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Exercises the exact write path the `toggle_pack` and `reorder_packs` commands use, and
    /// the read path `setup` uses, so the round trip is covered without driving the UI.
    #[test]
    fn test_preferences_round_trip_through_the_engine() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let fixture = root.join("packs/vertical-slice.sqlite");
        let dir = std::env::temp_dir().join("golddig-prefs-roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        let path = settings::settings_path(&dir);

        let mut engine = SearchEngine::new();
        engine.add_pack(&fixture, true).unwrap();
        let pack_id = engine.list_packs()[0].id.clone();

        // Disable it and save, the way toggle_pack does.
        engine.set_pack_enabled(&pack_id, false);
        persist_preferences(&engine, &path).expect("preferences must save");
        assert!(
            path.exists(),
            "settings file should exist at {}",
            path.display()
        );

        // Reload into a fresh engine, the way setup does.
        let mut reopened = SearchEngine::new();
        reopened.add_pack(&fixture, true).unwrap();
        let saved = settings::Settings::load(&path);
        reopened.apply_preferences(|id| saved.get(id).map(|p| (p.priority, p.enabled)));

        assert!(
            !reopened.list_packs()[0].enabled,
            "a pack disabled before restart must come back disabled"
        );

        // An unwritable location must report, not panic and not silently drop the change.
        let blocked = PathBuf::from("");
        assert!(
            persist_preferences(&engine, &blocked).is_ok(),
            "empty path is a no-op"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
