pub mod importers;
pub mod model;
pub mod normalization;
pub mod pack;
pub mod search;

use model::{EntryRecord, SearchSuggestion};
use search::SearchEngine;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pack_path = PathBuf::from("packs/vertical-slice.sqlite");
    let initial_engine = if pack_path.exists() {
        SearchEngine::open_pack(pack_path).ok()
    } else {
        None
    };

    tauri::Builder::default()
        .manage(AppState {
            engine: Mutex::new(initial_engine),
        })
        .invoke_handler(tauri::generate_handler![suggest, get_entry])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
