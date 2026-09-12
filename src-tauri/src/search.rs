// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{EntryRecord, PackManifest, SearchSuggestion};
use crate::normalization::generate_search_keys;
use rusqlite::{params, Connection, Result};
use std::path::{Path, PathBuf};

/// One ranked candidate from one pack, before the global merge.
struct Hit {
    tier: i64,
    term_len: usize,
    pack_idx: usize,
    suggestion: SearchSuggestion,
}

/// Smallest string greater than every string starting with `prefix`, giving an
/// index-usable half-open range `[prefix, upper)` in place of `LIKE 'prefix%'`.
fn prefix_upper_bound(prefix: &str) -> Option<String> {
    let mut chars: Vec<char> = prefix.chars().collect();
    while let Some(last) = chars.pop() {
        for next in (last as u32 + 1)..=char::MAX as u32 {
            if let Some(c) = char::from_u32(next) {
                let mut out: String = chars.iter().collect();
                out.push(c);
                return Some(out);
            }
        }
    }
    None
}

/// The literal tail of a wildcard pattern, when the pattern ends in one and every wildcard
/// sits before it — `*ight` -> `ight`, `l*ght` -> None (it already has a usable literal
/// prefix), `*igh*` -> None (no literal tail to anchor on).
///
/// A pattern like this is the only shape a forward index cannot serve at all, and the one
/// the reversed-term index exists for.
fn literal_suffix(query: &str) -> Option<String> {
    let first = query.chars().next()?;
    if first != '*' && first != '?' {
        return None; // a literal prefix exists; the forward index handles it
    }
    let tail: String = query
        .chars()
        .rev()
        .take_while(|c| *c != '*' && *c != '?')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    // Require a few characters, or the range probe degenerates into a scan anyway.
    if tail.chars().count() >= 2 {
        Some(tail)
    } else {
        None
    }
}

/// Translates the documented user syntax into a GLOB pattern. `?` and `*` already mean
/// what GLOB means, so only `[` needs neutralizing — GLOB reads `[...]` as a character
/// class, which would otherwise let a query silently become a different pattern.
fn to_glob_pattern(query: &str) -> String {
    query.replace('[', "[[]")
}

/// Escapes LIKE metacharacters so a literal `%` or `_` typed by the user stays literal.
/// Pair with an `ESCAPE` clause naming the same backslash in the SQL.
fn escape_like(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

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
    /// Counted once at open time. list_packs used to re-run COUNT(*) over every pack
    /// on mount and after every toggle.
    pub entry_count: usize,
    /// Whether this pack carries the reversed-term index used to serve suffix patterns.
    /// Packs built before it was added do not, and must still open and work — a format
    /// addition should never make an installed dictionary unreadable.
    pub has_term_rev: bool,
}

/// A pack that could not be opened. Surfaced to the UI so a corrupt or missing pack
/// is visible instead of silently producing an empty dictionary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackLoadError {
    pub path: String,
    pub message: String,
}

#[derive(Default)]
pub struct SearchEngine {
    packs: Vec<PackHandle>,
    load_errors: Vec<PackLoadError>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            packs: Vec::new(),
            load_errors: Vec::new(),
        }
    }

    /// Packs that failed to open, in discovery order.
    pub fn load_errors(&self) -> &[PackLoadError] {
        &self.load_errors
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

        // Several candidate directories are searched (bundled resources, app data, CWD)
        // and they can resolve to the same file in a dev checkout. Loading a pack twice
        // would double every suggestion.
        let canonical = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
        if self
            .packs
            .iter()
            .any(|h| std::fs::canonicalize(&h.path).unwrap_or_else(|_| h.path.clone()) == canonical)
        {
            return Ok(());
        }

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

        // Terms are stored lowercased by the builder and queries are lowercased before
        // binding, so making LIKE case-sensitive is semantically a no-op — but it is what
        // allows SQLite to use idx_search_term. With the default case-insensitive LIKE,
        // a BINARY index is ignored and every keystroke full-scans search_terms.
        conn.execute_batch("PRAGMA case_sensitive_like = ON;")?;

        let entry_count: i64 = conn
            .query_row("SELECT count(*) FROM entries", [], |r| r.get(0))
            .unwrap_or(0);

        let has_term_rev: bool = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('search_terms') WHERE name = 'term_rev'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n > 0)
            .unwrap_or(false);

        self.packs.push(PackHandle {
            manifest,
            path: p,
            conn,
            enabled,
            entry_count: entry_count as usize,
            has_term_rev,
        });

        Ok(())
    }

    /// Discovers and loads all .sqlite packs in a directory. Failures are recorded in
    /// `load_errors` rather than discarded.
    pub fn load_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<usize> {
        let mut count = 0;
        let d = dir.as_ref();
        if !d.is_dir() {
            self.load_errors.push(PackLoadError {
                path: d.to_string_lossy().to_string(),
                message: "pack directory not found".to_string(),
            });
            return Ok(0);
        }
        let read = match std::fs::read_dir(d) {
            Ok(r) => r,
            Err(e) => {
                self.load_errors.push(PackLoadError {
                    path: d.to_string_lossy().to_string(),
                    message: e.to_string(),
                });
                return Ok(0);
            }
        };
        let mut paths: Vec<PathBuf> = read
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("sqlite"))
            .collect();
        paths.sort();
        for path in paths {
            match self.add_pack(&path, true) {
                Ok(()) => count += 1,
                Err(e) => self.load_errors.push(PackLoadError {
                    path: path.to_string_lossy().to_string(),
                    message: e.to_string(),
                }),
            }
        }
        Ok(count)
    }

    /// Lists loaded packs and their status.
    pub fn list_packs(&self) -> Vec<PackInfo> {
        self.packs
            .iter()
            .map(|p| PackInfo {
                id: p.manifest.id.clone(),
                name: p.manifest.name.clone(),
                version: p.manifest.version.clone(),
                languages: p.manifest.languages.clone(),
                path: p.path.to_string_lossy().to_string(),
                enabled: p.enabled,
                entry_count: p.entry_count,
            })
            .collect()
    }

    /// Distinct languages across enabled packs, used to pick which loose-key rules to
    /// apply to a query.
    fn enabled_languages(&self) -> Vec<String> {
        let mut langs: Vec<String> = Vec::new();
        for p in self.packs.iter().filter(|p| p.enabled) {
            for l in &p.manifest.languages {
                if !langs.contains(l) {
                    langs.push(l.clone());
                }
            }
        }
        if langs.is_empty() {
            langs.push("generic".to_string());
        }
        langs
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

    /// Searches across all enabled packs and returns globally ranked suggestions.
    ///
    /// Query syntax: `?` matches exactly one character, `*` matches zero or more. Both
    /// map straight onto SQLite GLOB, which — unlike LIKE — is case-sensitive and can
    /// therefore use `idx_search_term` whenever the pattern has a literal prefix.
    ///
    /// Ranking is global. Each pack contributes candidates tagged with a tier and the
    /// merged list is sorted before truncation. Ranking within each pack and then
    /// concatenating (the previous behaviour) let a weak match in an alphabetically
    /// earlier pack outrank an exact lemma in a later one — searching `man` returned
    /// Arabic form matches above the English headword.
    pub fn suggest(&self, query: &str, limit: usize) -> Result<Vec<SearchSuggestion>> {
        let q_clean = query.trim();
        if q_clean.is_empty() {
            return Ok(Vec::new());
        }
        let q = q_clean.to_lowercase();

        let has_wildcard = q.contains('?') || q.contains('*');
        let has_space = q.contains(' ');

        const SELECT: &str = "SELECT e.id, e.lemma, e.language, e.pos, s.term, s.term_type \
             FROM search_terms s JOIN entries e ON e.id = s.entry_id ";

        // Tiers. Multiplied by 10 and offset by match type in push_hits, so an exact
        // lemma beats an exact form, which beats any prefix match.
        const TIER_EXACT: i64 = 0;
        const TIER_WILDCARD: i64 = 1;
        const TIER_PREFIX: i64 = 2;
        const TIER_LOOSE_EXACT: i64 = 3;
        const TIER_LOOSE_PREFIX: i64 = 4;
        const TIER_TERM_CONTAINS: i64 = 5;
        const TIER_CONTENT: i64 = 6;

        // Wall-clock budget for each unindexed scan, per pack, so worst-case latency is
        // bounded. Partial results are kept rather than discarded.
        const SCAN_BUDGET: std::time::Duration = std::time::Duration::from_millis(120);

        let mut hits: Vec<Hit> = Vec::new();
        // Over-fetch per pack so the global sort has real candidates to choose between.
        let per_pack = ((limit * 4).max(20)) as i64;

        // One loose key per distinct enabled language. The old code hardcoded
        // generic/Arabic/Chinese, so the Spanish, Catalan, German and French rules in
        // normalization.rs were never applied to a query.
        let mut loose_patterns: Vec<String> = Vec::new();
        for lang in self.enabled_languages() {
            let key = generate_search_keys(q_clean, &lang).loose_key;
            if !key.is_empty() && !loose_patterns.contains(&key) {
                loose_patterns.push(key);
            }
        }

        for (pack_idx, pack) in self.packs.iter().enumerate() {
            if !pack.enabled {
                continue;
            }

            if has_wildcard {
                let pattern = to_glob_pattern(&q);
                // A pattern whose wildcards are all leading (`*ight`) has a literal
                // *suffix*, which no forward b-tree can serve — it cost 589 ms mean and
                // 1.4 s p95 on a 198k-entry pack. Probe the reversed-term index instead and
                // still confirm with GLOB, so the match stays exact.
                let reverse_probe = if pack.has_term_rev {
                    literal_suffix(&q).and_then(|suffix| {
                        let reversed: String = suffix.chars().rev().collect();
                        prefix_upper_bound(&reversed).map(|hi| (reversed, hi))
                    })
                } else {
                    None
                };

                match reverse_probe {
                    Some((reversed, hi)) => Self::push_hits(
                        &pack.conn,
                        &mut hits,
                        pack_idx,
                        &format!(
                            "{SELECT} WHERE s.term_rev >= ?1 AND s.term_rev < ?2 \
                             AND s.term GLOB ?3 LIMIT ?4"
                        ),
                        &[&reversed, &hi, &pattern, &per_pack],
                        TIER_WILDCARD,
                    )?,
                    None => Self::push_hits(
                        &pack.conn,
                        &mut hits,
                        pack_idx,
                        &format!("{SELECT} WHERE s.term GLOB ?1 LIMIT ?2"),
                        &[&pattern, &per_pack],
                        TIER_WILDCARD,
                    )?,
                }
            } else {
                Self::push_hits(
                    &pack.conn,
                    &mut hits,
                    pack_idx,
                    &format!("{SELECT} WHERE s.term = ?1 LIMIT ?2"),
                    &[&q, &per_pack],
                    TIER_EXACT,
                )?;
                // Range bounds rather than LIKE 'x%': index-usable unconditionally.
                if let Some(hi) = prefix_upper_bound(&q) {
                    Self::push_hits(
                        &pack.conn,
                        &mut hits,
                        pack_idx,
                        &format!("{SELECT} WHERE s.term >= ?1 AND s.term < ?2 LIMIT ?3"),
                        &[&q, &hi, &per_pack],
                        TIER_PREFIX,
                    )?;
                }
            }

            // The loose/diacritic-folded tier always runs. It used to fire only when
            // every pack returned zero rows, so the documented collision guarantees
            // (ñ, l·l, ß, œ) silently stopped holding as soon as a second pack existed.
            for lp in &loose_patterns {
                Self::push_hits(
                    &pack.conn,
                    &mut hits,
                    pack_idx,
                    &format!("{SELECT} WHERE s.loose_key = ?1 LIMIT ?2"),
                    &[lp, &per_pack],
                    TIER_LOOSE_EXACT,
                )?;
                if let Some(hi) = prefix_upper_bound(lp) {
                    Self::push_hits(
                        &pack.conn,
                        &mut hits,
                        pack_idx,
                        &format!("{SELECT} WHERE s.loose_key >= ?1 AND s.loose_key < ?2 LIMIT ?3"),
                        &[lp, &hi, &per_pack],
                        TIER_LOOSE_PREFIX,
                    )?;
                }
            }

            // Multi-word queries also match inside a headword or collocation term. No index
            // can serve a leading `%`, so this runs under a wall-clock budget.
            if has_space {
                let pattern = format!("%{}%", escape_like(&q));
                Self::push_hits_timeboxed(
                    &pack.conn,
                    &mut hits,
                    pack_idx,
                    &format!("{SELECT} WHERE s.term LIKE ?1 ESCAPE '\\' LIMIT ?2"),
                    &[&pattern, &per_pack],
                    TIER_TERM_CONTAINS,
                    SCAN_BUDGET,
                )?;
            }
        }

        // Content search over definitions and examples. This is an unindexed scan of the
        // JSON payload (~79 ms on the 77 MB English pack), so it runs only for multi-word
        // queries that the indexed tiers could not satisfy — never on the keystroke path.
        if has_space && hits.len() < limit {
            let pattern = format!("%{}%", escape_like(&q));
            for (pack_idx, pack) in self.packs.iter().enumerate() {
                if !pack.enabled {
                    continue;
                }
                // Stop as soon as the list is full — unbudgeted, this scan read ~2 GB of
                // JSON per pack and measured 10.5 s mean / 18 s p95 on the English pack.
                if hits.len() >= limit {
                    break;
                }
                Self::push_hits_timeboxed(
                    &pack.conn,
                    &mut hits,
                    pack_idx,
                    "SELECT id, lemma, language, pos, lemma AS term, 'content' AS term_type \
                     FROM entries WHERE data_json LIKE ?1 ESCAPE '\\' LIMIT ?2",
                    &[&pattern, &per_pack],
                    TIER_CONTENT,
                    SCAN_BUDGET,
                )?;
            }
        }

        hits.sort_by(|a, b| {
            a.tier
                .cmp(&b.tier)
                .then(a.term_len.cmp(&b.term_len))
                .then(a.pack_idx.cmp(&b.pack_idx))
                .then_with(|| a.suggestion.lemma.cmp(&b.suggestion.lemma))
        });

        // Deduplicate on entry_id alone, keeping the best-ranked match. Keying on
        // (entry_id, match_type) listed the same entry once per match type — a search
        // for `hello` returned two rows, both the same word.
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::with_capacity(limit);
        for hit in hits {
            if seen.insert(hit.suggestion.entry_id.clone()) {
                out.push(hit.suggestion);
                if out.len() >= limit {
                    break;
                }
            }
        }

        Ok(out)
    }

    /// Runs an unindexed query under a wall-clock budget, keeping whatever it found.
    ///
    /// The two scan-shaped tiers (multi-word `LIKE '%…%'` over terms, and the content scan
    /// over `entries.data_json`) cannot use an index. On the 1.49M-entry English pack the
    /// content scan reads ~2 GB of JSON and took **10.5 s mean, 18 s p95** — reachable by
    /// typing any two words. A watchdog interrupts the statement at the deadline and the
    /// partial result is used, so worst-case latency is bounded instead of unbounded.
    ///
    /// `SQLITE_INTERRUPT` is the expected outcome here, not an error. Replacing this with a
    /// real FTS5 index over definitions and examples is the first item in docs/roadmap.md.
    fn push_hits_timeboxed(
        conn: &Connection,
        hits: &mut Vec<Hit>,
        pack_idx: usize,
        sql: &str,
        args: &[&dyn rusqlite::ToSql],
        base_tier: i64,
        budget: std::time::Duration,
    ) -> Result<()> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let handle = conn.get_interrupt_handle();
        let finished = Arc::new(AtomicBool::new(false));
        let watchdog_flag = Arc::clone(&finished);
        let watchdog = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            while !watchdog_flag.load(Ordering::Relaxed) {
                if start.elapsed() >= budget {
                    handle.interrupt();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });

        let result = Self::push_hits(conn, hits, pack_idx, sql, args, base_tier);

        finished.store(true, Ordering::Relaxed);
        let _ = watchdog.join();

        match result {
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::OperationInterrupted =>
            {
                Ok(()) // budget spent; keep the partial result
            }
            other => other,
        }
    }

    /// Runs one candidate query and appends its rows with a computed rank.
    fn push_hits(
        conn: &Connection,
        hits: &mut Vec<Hit>,
        pack_idx: usize,
        sql: &str,
        args: &[&dyn rusqlite::ToSql],
        base_tier: i64,
    ) -> Result<()> {
        let mut stmt = conn.prepare_cached(sql)?;
        let rows = stmt.query_map(args, |row| {
            Ok(SearchSuggestion {
                entry_id: row.get(0)?,
                lemma: row.get(1)?,
                language: row.get(2)?,
                pos: row.get(3)?,
                matched_term: row.get(4)?,
                match_type: row.get(5)?,
            })
        })?;
        for row in rows {
            let suggestion = row?;
            let type_rank = match suggestion.match_type.as_str() {
                "lemma" => 0,
                "transcription" => 1,
                "form" => 2,
                _ => 3,
            };
            hits.push(Hit {
                tier: base_tier * 10 + type_rank,
                term_len: suggestion.matched_term.chars().count(),
                pack_idx,
                suggestion,
            });
        }
        Ok(())
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
        assert!(
            !res_single.is_empty(),
            "Wildcard '?' lookup must find 'light'"
        );
        assert_eq!(res_single[0].lemma, "light");

        // '*' replaces zero or more characters: l*t -> light
        let res_multi = engine.suggest("l*t", 5).unwrap();
        assert!(
            !res_multi.is_empty(),
            "Wildcard '*' lookup must find 'light'"
        );
        assert!(res_multi.iter().any(|s| s.lemma == "light"));

        // Phrase lookup: "morning light"
        let res_phrase = engine.suggest("morning light", 5).unwrap();
        assert!(!res_phrase.is_empty(), "Phrase search must find 'light'");
        assert_eq!(res_phrase[0].lemma, "light");
    }

    /// Writes a minimal pack to a temp path. Used by the ranking tests so they do not
    /// depend on which real packs happen to be present.
    fn build_tiny_pack(tag: &str, pack_id: &str, entries_json: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("golddig-test-{tag}"));
        std::fs::create_dir_all(&dir).unwrap();

        let manifest = dir.join("manifest.json");
        std::fs::write(
            &manifest,
            format!(
                r#"{{"id":"{pack_id}","name":"{pack_id}","version":"1.0.0","schema_version":1,
                    "languages":["en","ary"],"created_at":"2026-01-01T00:00:00Z",
                    "license":"CC0-1.0","sources":[]}}"#
            ),
        )
        .unwrap();

        let entries = dir.join("entries.jsonl");
        std::fs::write(&entries, entries_json.join("\n")).unwrap();

        let out = dir.join("pack.sqlite");
        let _ = std::fs::remove_file(&out);
        crate::pack::build_pack_from_files(manifest.as_path(), entries.as_path(), out.as_path())
            .unwrap();
        out
    }

    /// One JSONL record on a single line — build_pack_from_files reads line by line.
    fn entry_json(id: &str, lang: &str, lemma: &str, forms: &[&str]) -> String {
        let forms_json: Vec<String> = forms
            .iter()
            .map(|f| format!(r#"{{"form":"{f}","type":"plural","source_id":"t"}}"#))
            .collect();
        format!(
            concat!(
                r#"{{"id":"{}","language":"{}","lemma":"{}","pos":"noun","#,
                r#""pronunciations":[],"#,
                r#""senses":[{{"id":"{}-s1","definition":"test sense","source_id":"t"}}],"#,
                r#""forms":[{}]}}"#
            ),
            id,
            lang,
            lemma,
            id,
            forms_json.join(",")
        )
    }

    /// The regression this whole engine rewrite exists for. Results used to be ranked
    /// within each pack and then concatenated in directory order, so a weak match in an
    /// alphabetically earlier pack outranked an exact headword in a later one: searching
    /// `man` returned Arabic inflected forms above the English word "man".
    #[test]
    fn test_exact_lemma_outranks_form_match_in_an_earlier_pack() {
        let a = build_tiny_pack(
            "rank-a",
            "aaa-first-pack",
            &[&entry_json(
                "ary:noun:mnshar",
                "ary",
                "mnshar",
                &["man-shar", "mandil"],
            )],
        );
        let b = build_tiny_pack(
            "rank-b",
            "zzz-second-pack",
            &[&entry_json("en:noun:man", "en", "man", &["men"])],
        );

        let mut engine = SearchEngine::new();
        engine.add_pack(&a, true).unwrap(); // loaded first, as sorting would
        engine.add_pack(&b, true).unwrap();

        let hits = engine.suggest("man", 10).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(
            hits[0].lemma,
            "man",
            "an exact lemma in a later pack must outrank a form match in an earlier one, got {:?}",
            hits.iter()
                .map(|h| (&h.lemma, &h.match_type))
                .collect::<Vec<_>>()
        );
        assert_eq!(hits[0].match_type, "lemma");
    }

    /// Deduplication keyed on (entry_id, match_type) listed one entry once per match
    /// type, so searching `man` spent two of its result slots on the same word.
    #[test]
    fn test_suggestions_never_repeat_an_entry() {
        let pack = build_tiny_pack(
            "dedupe",
            "dedupe-pack",
            &[&entry_json(
                "en:noun:light",
                "en",
                "light",
                &["lights", "lighting"],
            )],
        );
        let engine = SearchEngine::open_pack(&pack).unwrap();

        for query in ["light", "l*", "ligh"] {
            let hits = engine.suggest(query, 10).unwrap();
            let mut ids: Vec<&str> = hits.iter().map(|h| h.entry_id.as_str()).collect();
            let before = ids.len();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(
                before,
                ids.len(),
                "query {query:?} returned a duplicate entry"
            );
        }
    }

    /// A query containing both a space and a wildcard used to hit no branch at all:
    /// is_phrase was false because a wildcard was present, and the loose fallback was
    /// gated on !has_wildcard. Zero results, silently.
    #[test]
    fn test_wildcard_combined_with_space_still_matches() {
        let pack = build_tiny_pack(
            "wild-space",
            "wild-space-pack",
            &[&entry_json(
                "en:noun:morning-light",
                "en",
                "morning light",
                &[],
            )],
        );
        let engine = SearchEngine::open_pack(&pack).unwrap();

        for query in ["morning l*t", "morning li?ht", "mornin* light"] {
            let hits = engine.suggest(query, 5).unwrap();
            assert!(
                hits.iter().any(|h| h.lemma == "morning light"),
                "query {query:?} should match the multi-word headword"
            );
        }
    }

    /// LIKE metacharacters typed by the user must stay literal; `%` used to match every
    /// term in the dictionary and `_` acted as a single-character wildcard.
    #[test]
    fn test_like_metacharacters_are_literal() {
        let pack = build_tiny_pack(
            "escape",
            "escape-pack",
            &[
                &entry_json("en:noun:code", "en", "code", &[]),
                &entry_json("en:noun:c_de", "en", "c_de", &[]),
            ],
        );
        let engine = SearchEngine::open_pack(&pack).unwrap();

        // `_` is not in the documented syntax, so it must match only itself.
        let hits = engine.suggest("c_de", 10).unwrap();
        assert!(hits.iter().any(|h| h.lemma == "c_de"));
        assert!(
            !hits.iter().any(|h| h.lemma == "code"),
            "a literal underscore must not behave as a wildcard"
        );

        // A bare `%` is not a wildcard in the documented syntax either.
        let pct = engine.suggest("%", 10).unwrap();
        assert!(
            pct.is_empty(),
            "'%' must not match every headword, got {pct:?}"
        );
    }

    /// English had no diacritic folding at all: every language without a dedicated arm
    /// fell through to a loose_generic that only stripped punctuation.
    #[test]
    fn test_accent_free_typing_finds_accented_english_headword() {
        let pack = build_tiny_pack(
            "fold",
            "fold-pack",
            &[&entry_json("en:noun:fiancee", "en", "fiancée", &[])],
        );
        let engine = SearchEngine::open_pack(&pack).unwrap();
        let hits = engine.suggest("fiancee", 5).unwrap();
        assert!(
            hits.iter().any(|h| h.lemma == "fiancée"),
            "'fiancee' should reach 'fiancée'"
        );
    }

    /// The loose tier used to run only when every pack returned zero rows, so the
    /// documented collision guarantees stopped holding as soon as a second pack existed.
    #[test]
    fn test_loose_tier_runs_even_when_another_pack_matched() {
        let a = build_tiny_pack(
            "loose-a",
            "aaa-loose-pack",
            &[&entry_json("en:noun:strasser", "en", "strasser", &[])],
        );
        let b = build_tiny_pack(
            "loose-b",
            "zzz-loose-pack",
            &[&entry_json("en:noun:strasse", "en", "straße", &[])],
        );
        let mut engine = SearchEngine::new();
        engine.add_pack(&a, true).unwrap();
        engine.add_pack(&b, true).unwrap();

        // "strasser" matches pack A exactly; "straße" must still surface via the loose key.
        let hits = engine.suggest("strasse", 10).unwrap();
        assert!(
            hits.iter().any(|h| h.lemma == "straße"),
            "loose ß/ss folding must apply even though another pack produced a hit: {:?}",
            hits.iter().map(|h| &h.lemma).collect::<Vec<_>>()
        );
    }

    /// Guards the performance fix. The suggest prefix probe must use an index; it used to
    /// full-scan search_terms on every keystroke because SQLite cannot use a BINARY index
    /// for the default case-insensitive LIKE.
    #[test]
    fn test_prefix_probe_uses_an_index() {
        let pack = build_tiny_pack(
            "plan",
            "plan-pack",
            &[&entry_json("en:noun:light", "en", "light", &["lights"])],
        );
        let engine = SearchEngine::open_pack(&pack).unwrap();
        let handle = &engine.packs[0];

        let plan: String = handle
            .conn
            .query_row(
                "EXPLAIN QUERY PLAN SELECT e.id FROM search_terms s \
                 JOIN entries e ON e.id = s.entry_id WHERE s.term >= 'lig' AND s.term < 'lih'",
                [],
                |r| r.get(3),
            )
            .unwrap();
        assert!(
            plan.contains("USING INDEX") || plan.contains("USING COVERING INDEX"),
            "prefix probe must use an index, plan was: {plan}"
        );
        assert!(
            !plan.contains("SCAN search_terms"),
            "prefix probe must not scan the table, plan was: {plan}"
        );
    }
}
