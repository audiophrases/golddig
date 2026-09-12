// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Per-pack reader preferences, persisted next to the app's data.
//!
//! Two things needed somewhere to live. Pack enable/disable was in-memory only, so the app
//! forgot it on every restart. And ranking ties between packs were settled by load order,
//! which is alphabetical and arbitrary: with eight packs enabled, `man` is an exact lemma in
//! English, German, French, Spanish and Chinese simultaneously, and which one a reader wants
//! first is a preference the engine cannot infer.
//!
//! This is a small JSON file rather than the `catalog.sqlite` that `docs/architecture.md`
//! proposes. That database would also hold history, bookmarks and a cross-pack term index;
//! none of those exist yet, and inventing the schema before they do would be speculative.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What the reader has chosen about one pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackPreference {
    /// Lower sorts first. Ties inside a match tier are broken by this.
    pub priority: i64,
    pub enabled: bool,
}

/// The whole settings file. Keyed by pack id so reordering or renaming files is harmless.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub packs: BTreeMap<String, PackPreference>,
}

impl Settings {
    /// Reads the settings file, returning defaults when it is absent or unreadable.
    ///
    /// A corrupt or partly-written settings file must never stop the dictionary from
    /// opening, so every failure here degrades to defaults rather than propagating.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    /// Writes the settings file, creating its directory. Writes to a temporary file and
    /// renames, so an interrupted save cannot leave a half-written file behind.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, json)?;
        std::fs::rename(&temp, path)?;
        Ok(())
    }

    pub fn get(&self, pack_id: &str) -> Option<&PackPreference> {
        self.packs.get(pack_id)
    }

    pub fn set_enabled(&mut self, pack_id: &str, enabled: bool) {
        let entry = self
            .packs
            .entry(pack_id.to_string())
            .or_insert(PackPreference {
                priority: i64::MAX,
                enabled,
            });
        entry.enabled = enabled;
    }

    /// Assigns priorities from an explicit order, so position in the list *is* the priority.
    pub fn set_order(&mut self, ordered_pack_ids: &[String]) {
        for (index, pack_id) in ordered_pack_ids.iter().enumerate() {
            let entry = self.packs.entry(pack_id.clone()).or_insert(PackPreference {
                priority: index as i64,
                enabled: true,
            });
            entry.priority = index as i64;
        }
    }
}

/// The settings file inside the app's per-user data directory.
pub fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("pack-settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_round_trip_and_tolerate_garbage() {
        let dir = std::env::temp_dir().join("golddig-settings-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = settings_path(&dir);

        let mut settings = Settings::default();
        settings.set_order(&["b-pack".to_string(), "a-pack".to_string()]);
        settings.set_enabled("a-pack", false);
        settings.save(&path).unwrap();

        let reloaded = Settings::load(&path);
        // Order is explicit, not alphabetical: b-pack was placed first.
        assert_eq!(reloaded.get("b-pack").unwrap().priority, 0);
        assert_eq!(reloaded.get("a-pack").unwrap().priority, 1);
        assert!(!reloaded.get("a-pack").unwrap().enabled);
        assert!(reloaded.get("b-pack").unwrap().enabled);

        // A corrupt file must degrade to defaults, never prevent the app from starting.
        std::fs::write(&path, "{ this is not json").unwrap();
        assert!(Settings::load(&path).packs.is_empty());

        // So must a missing one.
        std::fs::remove_file(&path).unwrap();
        assert!(Settings::load(&path).packs.is_empty());
    }
}
