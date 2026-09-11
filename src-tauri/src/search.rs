use crate::model::{EntryRecord, SearchSuggestion};
use crate::normalization::generate_search_keys;
use rusqlite::{params, Connection, Result};

pub struct SearchEngine {
    conn: Connection,
}

impl SearchEngine {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn open_pack<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?;
        Ok(Self::new(conn))
    }

    /// Searches for prefix or matching suggestions across lemmas and inflected forms.
    /// Orders by match type: exact > form > loose.
    pub fn suggest(&self, query: &str, limit: usize) -> Result<Vec<SearchSuggestion>> {
        let q_clean = query.trim();
        if q_clean.is_empty() {
            return Ok(Vec::new());
        }

        let q_exact = q_clean.to_lowercase();
        let q_prefix = format!("{}%", q_exact);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT e.id, e.lemma, e.language, e.pos, s.term, s.term_type
            FROM search_terms s
            JOIN entries e ON e.id = s.entry_id
            WHERE s.term LIKE ?1 OR s.loose_key LIKE ?1
            ORDER BY 
                CASE 
                    WHEN s.term = ?2 AND s.term_type = 'lemma' THEN 1
                    WHEN s.term LIKE ?1 AND s.term_type = 'lemma' THEN 2
                    WHEN s.term = ?2 AND s.term_type = 'form' THEN 3
                    WHEN s.term LIKE ?1 AND s.term_type = 'form' THEN 4
                    ELSE 5
                END,
                length(e.lemma) ASC
            LIMIT ?3
            "#,
        )?;

        let rows = stmt.query_map(params![q_prefix, q_exact, limit as i64], |row| {
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

        let mut suggestions = Vec::new();
        for s in rows {
            suggestions.push(s?);
        }

        // If nothing matched via standard prefix, attempt language-specific loose search fallback
        if suggestions.is_empty() {
            let loose = generate_search_keys(q_clean, "generic");
            let loose_prefix = format!("{}%", loose.loose_key);

            let mut loose_stmt = self.conn.prepare(
                r#"
                SELECT DISTINCT e.id, e.lemma, e.language, e.pos, s.term, 'loose' as term_type
                FROM search_terms s
                JOIN entries e ON e.id = s.entry_id
                WHERE s.loose_key LIKE ?1
                ORDER BY length(e.lemma) ASC
                LIMIT ?2
                "#,
            )?;

            let loose_rows = loose_stmt.query_map(params![loose_prefix, limit as i64], |row| {
                Ok(SearchSuggestion {
                    entry_id: row.get(0)?,
                    lemma: row.get(1)?,
                    language: row.get(2)?,
                    pos: row.get(3)?,
                    matched_term: row.get(4)?,
                    match_type: row.get(5)?,
                })
            })?;

            for s in loose_rows {
                suggestions.push(s?);
            }
        }

        Ok(suggestions)
    }

    /// Fetches the full detailed EntryRecord by entry ID.
    pub fn get_entry(&self, entry_id: &str) -> Result<Option<EntryRecord>> {
        let mut stmt = self
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
            Ok(Some(entry))
        } else {
            Ok(None)
        }
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
        assert_eq!(suggs[0].language, "en-US");

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
}
