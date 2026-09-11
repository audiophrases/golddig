use crate::model::{EntryRecord, PackManifest, SearchSuggestion};
use crate::normalization::generate_search_keys;
use rusqlite::{params, Connection, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub languages: Vec<String>,
    pub path: String,
    pub enabled: bool,
    pub entry_count: usize,
}

pub struct PackHandle {
    pub manifest: PackManifest,
    pub path: PathBuf,
    pub conn: Connection,
    pub enabled: bool,
}

pub struct SearchEngine {
    packs: Vec<PackHandle>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self { packs: Vec::new() }
    }

    /// Opens a single pack database and registers it.
    pub fn open_pack<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut engine = Self::new();
        engine.add_pack(path, true)?;
        Ok(engine)
    }

    /// Adds a pack from a SQLite database file path.
    pub fn add_pack<P: AsRef<Path>>(&mut self, path: P, enabled: bool) -> Result<()> {
        let p = path.as_ref().to_path_buf();
        let conn = Connection::open_with_flags(
            &p,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?;

        let manifest_json: String = conn.query_row(
            "SELECT value FROM manifest WHERE key = 'manifest'",
            [],
            |r| r.get(0),
        )?;
        let manifest: PackManifest = serde_json::from_str(&manifest_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        self.packs.push(PackHandle {
            manifest,
            path: p,
            conn,
            enabled,
        });

        Ok(())
    }

    /// Discovers and loads all .sqlite packs in a directory.
    pub fn load_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<usize> {
        let mut count = 0;
        let d = dir.as_ref();
        if d.is_dir() {
            if let Ok(entries) = std::fs::read_dir(d) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sqlite")
                    {
                        if self.add_pack(&path, true).is_ok() {
                            count += 1;
                        }
                    }
                }
            }
        }
        Ok(count)
    }

    /// Lists loaded packs and their status.
    pub fn list_packs(&self) -> Vec<PackInfo> {
        self.packs
            .iter()
            .map(|p| {
                let count: i64 = p
                    .conn
                    .query_row("SELECT count(*) FROM entries", [], |r| r.get(0))
                    .unwrap_or(0);
                PackInfo {
                    id: p.manifest.id.clone(),
                    name: p.manifest.name.clone(),
                    version: p.manifest.version.clone(),
                    languages: p.manifest.languages.clone(),
                    path: p.path.to_string_lossy().to_string(),
                    enabled: p.enabled,
                    entry_count: count as usize,
                }
            })
            .collect()
    }

    /// Toggles a pack's active state by pack ID.
    pub fn set_pack_enabled(&mut self, pack_id: &str, enabled: bool) -> bool {
        for p in &mut self.packs {
            if p.manifest.id == pack_id {
                p.enabled = enabled;
                return true;
            }
        }
        false
    }

    /// Searches for prefix, wildcard ('?' or '*'), or phrase matching suggestions across all active packs.
    /// Orders by match type: exact > form > loose.
    pub fn suggest(&self, query: &str, limit: usize) -> Result<Vec<SearchSuggestion>> {
        let q_clean = query.trim();
        if q_clean.is_empty() {
            return Ok(Vec::new());
        }

        let q_lower = q_clean.to_lowercase();
        let has_wildcard = q_clean.contains('?') || q_clean.contains('*');
        let is_phrase = q_clean.contains(' ') && !has_wildcard;

        // Convert user wildcards ('?' -> '_', '*' -> '%') or build prefix/phrase pattern
        let sql_pattern = if has_wildcard {
            q_lower.replace('*', "%").replace('?', "_")
        } else if is_phrase {
            // Phrase lookup: look for multi-word phrase matching lemma/term
            format!("%{}%", q_lower)
        } else {
            // Standard word prefix lookup
            format!("{}%", q_lower)
        };

        let mut all_suggestions = Vec::new();

        for pack in &self.packs {
            if !pack.enabled {
                continue;
            }

            let mut stmt = pack.conn.prepare(
                r#"
                SELECT DISTINCT e.id, e.lemma, e.language, e.pos, s.term, s.term_type
                FROM search_terms s
                JOIN entries e ON s.entry_id = e.id
                WHERE s.term LIKE ?1
                ORDER BY
                    CASE
                        WHEN s.term = ?2 AND s.term_type = 'lemma' THEN 1
                        WHEN s.term LIKE ?1 AND s.term_type = 'lemma' THEN 2
                        WHEN s.term = ?2 AND s.term_type = 'transcription' THEN 3
                        WHEN s.term = ?2 AND s.term_type = 'form' THEN 4
                        WHEN s.term LIKE ?1 AND s.term_type = 'transcription' THEN 5
                        WHEN s.term LIKE ?1 AND s.term_type = 'form' THEN 6
                        ELSE 7
                    END,
                    length(s.term) ASC
                LIMIT ?3
                "#,
            )?;

            let rows = stmt.query_map(params![sql_pattern, q_lower, limit as i64], |row| {
                let entry_id: String = row.get(0)?;
                let lemma: String = row.get(1)?;
                let language: String = row.get(2)?;
                let pos: Option<String> = row.get(3)?;
                let matched_term: String = row.get(4)?;
                let match_type: String = row.get(5)?;

                Ok(SearchSuggestion {
                    entry_id,
                    lemma,
                    language,
                    pos,
                    matched_term,
                    match_type,
                })
            })?;

            for s in rows {
                all_suggestions.push(s?);
            }
        }

        // If nothing matched via standard search or wildcards, attempt phrase/content search or loose search
        if all_suggestions.is_empty() {
            if is_phrase {
                // Secondary phrase search inside example sentences or definitions across entries
                let phrase_pattern = format!("%{}%", q_lower);
                for pack in &self.packs {
                    if !pack.enabled {
                        continue;
                    }

                    let mut phrase_stmt = pack.conn.prepare(
                        r#"
                        SELECT DISTINCT id, lemma, language, pos, lemma as term, 'phrase' as term_type
                        FROM entries
                        WHERE data_json LIKE ?1
                        LIMIT ?2
                        "#,
                    )?;

                    let phrase_rows = phrase_stmt.query_map(params![phrase_pattern, limit as i64], |row| {
                        Ok(SearchSuggestion {
                            entry_id: row.get(0)?,
                            lemma: row.get(1)?,
                            language: row.get(2)?,
                            pos: row.get(3)?,
                            matched_term: row.get(4)?,
                            match_type: row.get(5)?,
                        })
                    })?;

                    for s in phrase_rows {
                        all_suggestions.push(s?);
                    }
                }
            } else if !has_wildcard {
                // Check loose forms across standard and specialized language rules (Spanish, Arabic, Chinese/Pinyin)
                let loose = generate_search_keys(q_clean, "generic");
                let loose_ar = generate_search_keys(q_clean, "ary");
                let loose_zh = generate_search_keys(q_clean, "zh");

                for pack in &self.packs {
                    if !pack.enabled {
                        continue;
                    }

                    let mut loose_stmt = pack.conn.prepare(
                        r#"
                        SELECT DISTINCT e.id, e.lemma, e.language, e.pos, s.term, 'loose' as term_type
                        FROM search_terms s
                        JOIN entries e ON e.id = s.entry_id
                        WHERE s.loose_key LIKE ?1 OR s.loose_key LIKE ?2 OR s.loose_key LIKE ?3
                        ORDER BY length(e.lemma) ASC
                        LIMIT ?4
                        "#,
                    )?;

                    let loose_rows = loose_stmt.query_map(
                        params![
                            format!("{}%", loose.loose_key),
                            format!("{}%", loose_ar.loose_key),
                            format!("{}%", loose_zh.loose_key),
                            limit as i64
                        ],
                        |row| {
                            Ok(SearchSuggestion {
                                entry_id: row.get(0)?,
                                lemma: row.get(1)?,
                                language: row.get(2)?,
                                pos: row.get(3)?,
                                matched_term: row.get(4)?,
                                match_type: row.get(5)?,
                            })
                        },
                    )?;

                    for s in loose_rows {
                        all_suggestions.push(s?);
                    }
                }
            }
        }

        // Deduplicate suggestions by (entry_id, match_type) while maintaining priority order
        let mut seen = std::collections::HashSet::new();
        let mut deduplicated = Vec::new();
        for item in all_suggestions {
            let key = (item.entry_id.clone(), item.match_type.clone());
            if !seen.contains(&key) {
                seen.insert(key);
                deduplicated.push(item);
                if deduplicated.len() >= limit {
                    break;
                }
            }
        }

        Ok(deduplicated)
    }

    /// Fetches the full detailed EntryRecord by entry ID across all active packs.
    pub fn get_entry(&self, entry_id: &str) -> Result<Option<EntryRecord>> {
        for pack in &self.packs {
            if !pack.enabled {
                continue;
            }

            let mut stmt = pack
                .conn
                .prepare("SELECT data_json FROM entries WHERE id = ?1")?;
            let mut rows = stmt.query_map(params![entry_id], |row| {
                let json_str: String = row.get(0)?;
                Ok(json_str)
            })?;

            if let Some(res) = rows.next() {
                let json_str = res?;
                let entry: EntryRecord = serde_json::from_str(&json_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                return Ok(Some(entry));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_fixture_engine() -> SearchEngine {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let db_path = root.join("packs/vertical-slice.sqlite");
        SearchEngine::open_pack(db_path).expect("Fixture sqlite pack must exist")
    }

    #[test]
    fn test_exact_lookup_light() {
        let engine = get_fixture_engine();
        let suggs = engine.suggest("light", 5).unwrap();
        assert!(!suggs.is_empty());
        assert_eq!(suggs[0].lemma, "light");
        assert_eq!(suggs[0].language, "en");

        let entry = engine.get_entry(&suggs[0].entry_id).unwrap().unwrap();
        assert_eq!(entry.lemma, "light");
        assert_eq!(entry.senses.len(), 1);
        assert_eq!(entry.senses[0].translations.len(), 4);
    }

    #[test]
    fn test_inflected_form_lights() {
        let engine = get_fixture_engine();
        let suggs = engine.suggest("lights", 5).unwrap();
        assert!(!suggs.is_empty());
        assert_eq!(suggs[0].lemma, "light");
        assert_eq!(suggs[0].match_type, "form");
    }

    #[test]
    fn test_spanish_ano_and_anyo_collision() {
        let engine = get_fixture_engine();

        // Looking up "ano" must return "ano" first, NOT "año"
        let ano_suggs = engine.suggest("ano", 5).unwrap();
        assert!(!ano_suggs.is_empty());
        assert_eq!(ano_suggs[0].lemma, "ano");

        // Looking up "año" must return "año" first, NOT "ano"
        let anyo_suggs = engine.suggest("año", 5).unwrap();
        assert!(!anyo_suggs.is_empty());
        assert_eq!(anyo_suggs[0].lemma, "año");
    }

    #[test]
    fn test_catalan_ela_geminada_lookup() {
        let engine = get_fixture_engine();
        let suggs = engine.suggest("col·lecció", 5).unwrap();
        assert!(!suggs.is_empty());
        assert_eq!(suggs[0].lemma, "col·lecció");
    }

    #[test]
    fn test_german_strasse_lookup() {
        let engine = get_fixture_engine();
        let suggs = engine.suggest("Straße", 5).unwrap();
        assert!(!suggs.is_empty());
        assert_eq!(suggs[0].lemma, "Straße");
    }

    #[test]
    fn test_multi_pack_toggle_and_list() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let db_path = root.join("packs/vertical-slice.sqlite");

        let mut engine = SearchEngine::new();
        engine.add_pack(&db_path, true).unwrap();

        let packs = engine.list_packs();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].id, "golddig-vertical-slice-core");
        assert!(packs[0].enabled);
        assert!(packs[0].entry_count >= 5);

        // Turn pack off -> suggestions must be empty
        engine.set_pack_enabled("golddig-vertical-slice-core", false);
        let suggs = engine.suggest("light", 5).unwrap();
        assert!(suggs.is_empty(), "Disabled pack must yield no suggestions");

        // Turn pack on -> suggestions restore
        engine.set_pack_enabled("golddig-vertical-slice-core", true);
        let suggs2 = engine.suggest("light", 5).unwrap();
        assert!(!suggs2.is_empty(), "Re-enabled pack must yield suggestions");
    }

    #[test]
    fn test_search_wildcard_and_phrase_lookup() {
        let engine = get_fixture_engine();

        // '?' replaces single character: l?ght -> light
        let res_single = engine.suggest("l?ght", 5).unwrap();
        assert!(!res_single.is_empty(), "Wildcard '?' lookup must find 'light'");
        assert_eq!(res_single[0].lemma, "light");

        // '*' replaces zero or more characters: l*t -> light
        let res_multi = engine.suggest("l*t", 5).unwrap();
        assert!(!res_multi.is_empty(), "Wildcard '*' lookup must find 'light'");
        assert!(res_multi.iter().any(|s| s.lemma == "light"));

        // Phrase lookup: "morning light"
        let res_phrase = engine.suggest("morning light", 5).unwrap();
        assert!(!res_phrase.is_empty(), "Phrase search must find 'light'");
        assert_eq!(res_phrase[0].lemma, "light");
    }
}
