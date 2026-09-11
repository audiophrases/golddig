pub mod importers;
pub mod model;
pub mod normalization;
pub mod pack;
pub mod search;

use model::{EntryRecord, SearchSuggestion};
use search::{PackInfo, SearchEngine};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub engine: Mutex<Option<SearchEngine>>,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut engine = SearchEngine::new();
    let packs_dir = PathBuf::from("packs");
    if packs_dir.is_dir() {
        let _ = engine.load_directory(&packs_dir);
    } else {
        let pack_path = PathBuf::from("packs/vertical-slice.sqlite");
        if pack_path.exists() {
            let _ = engine.add_pack(pack_path, true);
        }
    }

    tauri::Builder::default()
        .manage(AppState {
            engine: Mutex::new(Some(engine)),
        })
        .invoke_handler(tauri::generate_handler![
            suggest,
            get_entry,
            list_packs,
            toggle_pack
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
