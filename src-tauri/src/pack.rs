// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{EntryRecord, ExampleRecord, PackManifest};
use crate::normalization::generate_search_keys;
use rusqlite::{params, Connection, Result};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Build in MEMORY journal mode and finalize to DELETE in finalize_pack(). A
        -- shipped pack must not be WAL: WAL needs a writable directory and a -shm
        -- sidecar, so a pack installed under Program Files could not be opened at all.
        PRAGMA journal_mode = MEMORY;
        PRAGMA synchronous = OFF;

        CREATE TABLE IF NOT EXISTS manifest (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sources (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            license TEXT NOT NULL,
            attribution TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS entries (
            id TEXT PRIMARY KEY,
            language TEXT NOT NULL,
            lemma TEXT NOT NULL,
            pos TEXT,
            data_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS search_terms (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_id TEXT NOT NULL REFERENCES entries(id),
            term TEXT NOT NULL,
            term_type TEXT NOT NULL, -- 'lemma', 'form', 'transcription'
            loose_key TEXT NOT NULL,
            -- term with its characters reversed, so a suffix pattern becomes a prefix
            -- probe. Without it a leading-wildcard query such as *ight cost 589 ms mean
            -- and 1.4 s p95 on a 198k-entry pack, because no b-tree can serve a suffix.
            term_rev TEXT NOT NULL DEFAULT '',
            language TEXT NOT NULL,
            UNIQUE (entry_id, term, term_type)
        );

        -- Covering indexes. The suggest query selects (term, term_type, entry_id), so
        -- these let SQLite answer a prefix probe from the index alone. They are only
        -- usable because the builder lowercases every term and the engine sets
        -- PRAGMA case_sensitive_like = ON; with the default case-insensitive LIKE,
        -- SQLite ignores a BINARY index and full-scans the table on every keystroke.
        CREATE INDEX IF NOT EXISTS idx_search_term
            ON search_terms(term, term_type, entry_id);
        CREATE INDEX IF NOT EXISTS idx_search_loose
            ON search_terms(loose_key, term_type, entry_id);
        CREATE INDEX IF NOT EXISTS idx_search_term_rev
            ON search_terms(term_rev, term_type, entry_id);
        CREATE INDEX IF NOT EXISTS idx_entries_lang ON entries(language);
        "#,
    )?;
    Ok(())
}

/// Statistics a pack build reports, so silent data loss becomes visible.
#[derive(Debug, Default, Clone)]
pub struct BuildStats {
    pub entries: usize,
    pub unparseable_lines: usize,
    pub skipped_no_senses: usize,
    pub id_collisions: usize,
    pub search_terms: usize,
    pub transcription_terms: usize,
}

impl std::fmt::Display for BuildStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} entries, {} search terms ({} transcription), \
             {} unparseable lines, {} without senses, {} id collisions disambiguated",
            self.entries,
            self.search_terms,
            self.transcription_terms,
            self.unparseable_lines,
            self.skipped_no_senses,
            self.id_collisions
        )
    }
}

/// Writes every search term for one entry: the lemma, each inflected form, and each
/// romanized transcription. Returns (terms written, transcription terms written).
fn insert_search_terms(
    tx: &rusqlite::Transaction<'_>,
    entry: &EntryRecord,
) -> Result<(usize, usize)> {
    let mut total = 0usize;
    let mut transcriptions = 0usize;

    let write = |term: &str, kind: &str| -> Result<usize> {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            return Ok(0);
        }
        let keys = generate_search_keys(trimmed, &entry.language);
        if keys.exact_normalized.is_empty() {
            return Ok(0);
        }
        let reversed: String = keys.exact_normalized.chars().rev().collect();
        let n = tx.execute(
            "INSERT OR IGNORE INTO search_terms \
             (entry_id, term, term_type, loose_key, term_rev, language) \
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                entry.id,
                keys.exact_normalized,
                kind,
                keys.loose_key,
                reversed,
                entry.language
            ],
        )?;
        Ok(n)
    };

    total += write(&entry.lemma, "lemma")?;

    for f in &entry.forms {
        // A "form" identical to the lemma adds nothing but a duplicate suggestion row.
        if f.form.trim() == entry.lemma.trim() {
            continue;
        }
        total += write(&f.form, "form")?;
    }

    // Romanized transcriptions, so Pinyin and Latinized Darija are searchable.
    for p in &entry.pronunciations {
        if p.kind == "transcription" || p.kind == "pinyin" {
            let n = write(&p.ipa, "transcription")?;
            total += n;
            transcriptions += n;
        }
    }
    // Forms the source tags as a romanization are transcriptions too.
    for f in &entry.forms {
        let k = f.kind.to_ascii_lowercase();
        if k.contains("romanization") || k.contains("pinyin") || k.contains("transliteration") {
            let n = write(&f.form, "transcription")?;
            total += n;
            transcriptions += n;
        }
    }

    Ok((total, transcriptions))
}

/// Turns the build database into a shippable artifact: no WAL sidecars, compacted,
/// and integrity-checked.
fn finalize_pack(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA journal_mode = DELETE;")?;
    conn.execute_batch("VACUUM;")?;
    conn.execute_batch("ANALYZE;")?;
    let quick_check: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))?;
    if quick_check != "ok" {
        anyhow::bail!("Pack SQLite corruption check failed: {}", quick_check);
    }
    let fk: i64 = conn.query_row("SELECT count(*) FROM pragma_foreign_key_check;", [], |r| {
        r.get(0)
    })?;
    if fk != 0 {
        anyhow::bail!("Pack has {} foreign key violations", fk);
    }
    Ok(())
}

pub fn build_pack_from_files<P: AsRef<Path>>(
    manifest_path: P,
    entries_path: P,
    output_db_path: P,
) -> anyhow::Result<()> {
    if output_db_path.as_ref().exists() {
        let _ = std::fs::remove_file(&output_db_path);
        let _ = std::fs::remove_file(format!("{}-wal", output_db_path.as_ref().display()));
        let _ = std::fs::remove_file(format!("{}-shm", output_db_path.as_ref().display()));
    }

    let mut conn = Connection::open(&output_db_path)?;
    init_schema(&conn)?;

    let manifest_file = File::open(manifest_path)?;
    let manifest: PackManifest = serde_json::from_reader(manifest_file)?;

    let tx = conn.transaction()?;

    tx.execute(
        "INSERT OR REPLACE INTO manifest (key, value) VALUES ('manifest', ?)",
        params![serde_json::to_string(&manifest)?],
    )?;

    for src in &manifest.sources {
        tx.execute(
            "INSERT OR REPLACE INTO sources (id, name, url, license, attribution) VALUES (?, ?, ?, ?, ?)",
            params![src.id, src.name, src.url, src.license, src.attribution],
        )?;
    }

    let entries_file = File::open(entries_path)?;
    let reader = BufReader::new(entries_file);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let entry: EntryRecord = serde_json::from_str(trimmed)?;
        let data_json = serde_json::to_string(&entry)?;

        tx.execute(
            "INSERT INTO entries (id, language, lemma, pos, data_json) VALUES (?, ?, ?, ?, ?)",
            params![entry.id, entry.language, entry.lemma, entry.pos, data_json],
        )?;

        insert_search_terms(&tx, &entry)?;
    }

    tx.commit()?;
    finalize_pack(&conn)?;

    Ok(())
}

pub fn build_pack_from_kaikki<P: AsRef<Path>>(
    manifest_path: P,
    kaikki_jsonl_path: P,
    output_db_path: P,
    source_id: &str,
    max_entries: Option<usize>,
) -> anyhow::Result<usize> {
    if output_db_path.as_ref().exists() {
        let _ = std::fs::remove_file(&output_db_path);
    }

    let mut conn = Connection::open(&output_db_path)?;
    init_schema(&conn)?;

    let manifest_file = File::open(manifest_path)?;
    let manifest: PackManifest = serde_json::from_reader(manifest_file)?;

    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO manifest (key, value) VALUES ('manifest', ?)",
        params![serde_json::to_string(&manifest)?],
    )?;

    for src in &manifest.sources {
        tx.execute(
            "INSERT INTO sources (id, name, url, license, attribution) VALUES (?, ?, ?, ?, ?)",
            params![src.id, src.name, src.url, src.license, src.attribution],
        )?;
    }

    let entries_file = File::open(kaikki_jsonl_path)?;
    let reader = BufReader::new(entries_file);

    let mut stats = BuildStats::default();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let raw_entry = match serde_json::from_str::<crate::importers::kaikki::KaikkiEntry>(trimmed)
        {
            Ok(e) => e,
            Err(_) => {
                stats.unparseable_lines += 1;
                continue;
            }
        };

        let Some(mut entry) = raw_entry.into_entry_record(source_id) else {
            stats.skipped_no_senses += 1;
            continue;
        };

        // Two source records can share (language, pos, lemma, etymology). The old code
        // used INSERT OR REPLACE, which silently destroyed the earlier entry while the
        // counter kept incrementing — the reported entry count exceeded the rows stored.
        let base_id = entry.id.clone();
        let mut suffix = 1;
        while tx.query_row(
            "SELECT 1 FROM entries WHERE id = ?1",
            params![entry.id],
            |_| Ok(()),
        ) == Ok(())
        {
            suffix += 1;
            entry.id = format!("{}~{}", base_id, suffix);
            stats.id_collisions += 1;
        }

        let data_json = serde_json::to_string(&entry)?;
        tx.execute(
            "INSERT INTO entries (id, language, lemma, pos, data_json) VALUES (?, ?, ?, ?, ?)",
            params![entry.id, entry.language, entry.lemma, entry.pos, data_json],
        )?;

        let (terms, transcriptions) = insert_search_terms(&tx, &entry)?;
        stats.search_terms += terms;
        stats.transcription_terms += transcriptions;

        stats.entries += 1;
        if let Some(max) = max_entries {
            if stats.entries >= max {
                break;
            }
        }
    }

    tx.commit()?;
    finalize_pack(&conn)?;

    // The counter and the table must agree, or the reported figure is fiction.
    let stored: usize = conn.query_row("SELECT count(*) FROM entries", [], |r| r.get(0))?;
    if stored != stats.entries {
        anyhow::bail!(
            "Pack entry count mismatch: counted {} but stored {}",
            stats.entries,
            stored
        );
    }

    println!("Build summary: {}", stats);
    Ok(stats.entries)
}

/// Golddig language code -> Tatoeba's ISO 639-3 code.
fn tatoeba_lang_code(lang: &str) -> &str {
    match lang {
        "en" => "eng",
        "ca" => "cat",
        "es" => "spa",
        "fr" => "fra",
        "de" => "deu",
        "zh" | "cmn" => "cmn",
        "ary" => "ary",
        other => other,
    }
}

/// Splits a sentence into lowercase word tokens for headword matching.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
        .filter(|t| t.chars().count() > 1)
        .map(|t| t.trim_matches(|c| c == '\'' || c == '-').to_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Merges Tatoeba example sentences into an existing pack, attaching each sentence to the
/// headwords it contains and carrying an English translation where one is linked.
///
/// Replaces what used to be a `println!("Tatoeba sentence pair pack ingestion mode ready.")`
/// that created nothing and exited 0 — which is why no shipped pack ever contained a single
/// Tatoeba sentence despite a commit titled "add Tatoeba bilingual sentence corpus
/// integration".
///
/// `sentences_path` and `links_path` are the raw Tatoeba TSV exports from
/// <https://tatoeba.org/downloads> (`sentences.csv`: id, lang, text; `links.csv`:
/// sentence_id, translation_id). Three streaming passes keep memory bounded; the full
/// sentence export is far too large to hold at once.
pub fn merge_tatoeba_examples<P: AsRef<Path>>(
    pack_path: P,
    sentences_path: P,
    links_path: P,
    lang: &str,
    max_per_entry: usize,
) -> anyhow::Result<usize> {
    let tatoeba_lang = tatoeba_lang_code(lang);
    println!("[tatoeba] target language {lang} (Tatoeba code {tatoeba_lang})");

    // Pass 1: sentences in the target language.
    let mut target: HashMap<u64, String> = HashMap::new();
    for line in BufReader::new(File::open(&sentences_path)?).lines() {
        let line = line?;
        let mut parts = line.split('\t');
        let (Some(id), Some(code), Some(text)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if code != tatoeba_lang {
            continue;
        }
        if let Ok(id) = id.parse::<u64>() {
            target.insert(id, text.to_string());
        }
    }
    println!("[tatoeba] {} sentences in {}", target.len(), tatoeba_lang);
    if target.is_empty() {
        anyhow::bail!(
            "no {tatoeba_lang} sentences found in {:?}",
            sentences_path.as_ref()
        );
    }

    // Pass 2: links from those sentences, so we know which translations to fetch.
    let mut translation_of: HashMap<u64, u64> = HashMap::new();
    let mut wanted: HashSet<u64> = HashSet::new();
    for line in BufReader::new(File::open(&links_path)?).lines() {
        let line = line?;
        let mut parts = line.split('\t');
        let (Some(a), Some(b)) = (parts.next(), parts.next()) else {
            continue;
        };
        let (Ok(a), Ok(b)) = (a.parse::<u64>(), b.parse::<u64>()) else {
            continue;
        };
        if target.contains_key(&a) {
            translation_of.entry(a).or_insert(b);
            wanted.insert(b);
        }
    }

    // Pass 3: the English text of those linked translations.
    let mut english: HashMap<u64, String> = HashMap::new();
    for line in BufReader::new(File::open(&sentences_path)?).lines() {
        let line = line?;
        let mut parts = line.split('\t');
        let (Some(id), Some(code), Some(text)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if code != "eng" {
            continue;
        }
        if let Ok(id) = id.parse::<u64>() {
            if wanted.contains(&id) {
                english.insert(id, text.to_string());
            }
        }
    }
    println!("[tatoeba] {} linked English translations", english.len());

    // Match each sentence's tokens against the pack's headwords and inflected forms.
    let mut conn = Connection::open(pack_path.as_ref())?;
    let mut pending: HashMap<String, Vec<ExampleRecord>> = HashMap::new();
    {
        let mut lookup = conn.prepare(
            "SELECT DISTINCT entry_id FROM search_terms \
             WHERE term = ?1 AND term_type IN ('lemma', 'form')",
        )?;
        for (sentence_id, text) in &target {
            let translation = translation_of
                .get(sentence_id)
                .and_then(|tid| english.get(tid))
                .cloned();

            let mut seen_tokens = HashSet::new();
            for token in tokenize(text) {
                if !seen_tokens.insert(token.clone()) {
                    continue;
                }
                let ids: Vec<String> = lookup
                    .query_map(params![token], |r| r.get::<_, String>(0))?
                    .filter_map(|r| r.ok())
                    .collect();
                for entry_id in ids {
                    let slot = pending.entry(entry_id).or_default();
                    if slot.len() >= max_per_entry {
                        continue;
                    }
                    slot.push(ExampleRecord {
                        text: text.clone(),
                        translation: translation.clone(),
                        roman: None,
                        source_id: format!("tatoeba:{sentence_id}"),
                    });
                }
            }
        }
    }
    println!("[tatoeba] matched sentences to {} entries", pending.len());

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT OR REPLACE INTO sources (id, name, url, license, attribution) \
         VALUES (?, ?, ?, ?, ?)",
        params![
            "tatoeba",
            "Tatoeba Project",
            "https://tatoeba.org",
            "CC-BY-2.0-FR",
            "Tatoeba contributors; the sentence ID is recorded on every imported example"
        ],
    )?;

    let mut added = 0usize;
    {
        let mut read = tx.prepare("SELECT data_json FROM entries WHERE id = ?1")?;
        let mut write = tx.prepare("UPDATE entries SET data_json = ?2 WHERE id = ?1")?;
        for (entry_id, examples) in &pending {
            let json: String = match read.query_row(params![entry_id], |r| r.get(0)) {
                Ok(j) => j,
                Err(_) => continue,
            };
            let mut entry: EntryRecord = serde_json::from_str(&json)?;
            if entry.senses.is_empty() {
                continue;
            }
            // Attach to the first sense. Tatoeba sentences are not sense-disambiguated, so
            // assigning one would be inventing precision the data does not carry.
            let existing: HashSet<String> = entry.senses[0]
                .examples
                .iter()
                .map(|e| e.text.clone())
                .collect();
            for example in examples {
                if existing.contains(&example.text) {
                    continue;
                }
                entry.senses[0].examples.push(example.clone());
                added += 1;
            }
            write.execute(params![entry_id, serde_json::to_string(&entry)?])?;
        }
    }
    tx.commit()?;
    finalize_pack(&conn)?;

    println!("[tatoeba] added {added} examples");
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_pack_builder_e2e() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let manifest = root.join("fixtures/vertical-slice.manifest.json");
        let entries = root.join("fixtures/vertical-slice.entries.jsonl");
        let temp_db = std::env::temp_dir().join("golddig_test_pack.sqlite");

        let res = build_pack_from_files(&manifest, &entries, &temp_db);
        assert!(res.is_ok(), "Build pack must succeed: {:?}", res.err());

        let conn = Connection::open(&temp_db).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert!(count >= 5, "Must insert at least 5 entries");

        let terms_count: i64 = conn
            .query_row("SELECT count(*) FROM search_terms", [], |r| r.get(0))
            .unwrap();
        assert!(terms_count >= 5, "Must index search terms");

        let _ = std::fs::remove_file(temp_db);
    }

    #[test]
    fn test_pack_builder_from_kaikki() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let manifest = root.join("fixtures/vertical-slice.manifest.json");
        let entries = root.join("fixtures/kaikki-sample-ca.jsonl");
        let temp_db = std::env::temp_dir().join("golddig_test_kaikki_pack.sqlite");

        let res = build_pack_from_kaikki(&manifest, &entries, &temp_db, "kaikki-wiktionary", None);
        assert!(
            res.is_ok(),
            "Build pack from Kaikki must succeed: {:?}",
            res.err()
        );
        let count = res.unwrap();
        assert!(count > 0, "Must import entries from real Kaikki sample");

        let conn = Connection::open(&temp_db).unwrap();
        let db_count: i64 = conn
            .query_row("SELECT count(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(db_count, count as i64);

        let _ = std::fs::remove_file(temp_db);
    }

    /// Structural invariants every shipped pack must satisfy.
    ///
    /// This is the test whose absence let the whole class of defect ship: 19 tests passed
    /// while the artifacts the user actually ran had 0% translations, 0% collocations, no
    /// transcription index, and a "Mandarin" pack containing no Chinese. Nothing ever
    /// opened a built pack.
    ///
    /// It runs over whatever is in packs/. CI builds the 6 MB Darija pack so this is never
    /// a silent no-op.
    #[test]
    fn test_shipped_packs_are_structurally_sound() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let packs_dir = root.join("packs");

        let mut packs: Vec<PathBuf> = std::fs::read_dir(&packs_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("sqlite"))
            .collect();
        packs.sort();
        assert!(
            !packs.is_empty(),
            "no packs in {} — the committed fixture must always be present",
            packs_dir.display()
        );

        for pack in packs {
            let name = pack.file_name().unwrap().to_string_lossy().to_string();
            let conn =
                Connection::open_with_flags(&pack, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap_or_else(|e| panic!("{name}: could not open read-only: {e}"));

            // A shipped pack must not be in WAL mode: WAL needs a writable directory and a
            // -shm sidecar, so a pack installed under Program Files would not open at all.
            // A pack being written right now is locked. Wait briefly, then say so plainly
            // rather than surfacing a bare "database is locked".
            conn.busy_timeout(std::time::Duration::from_secs(10)).ok();
            let journal: String = conn
                .query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .unwrap_or_else(|e| {
                    panic!(
                        "{name}: could not read journal_mode ({e}). If a pack build is \
                            running, wait for it to finish and re-run the tests."
                    )
                });
            assert_ne!(
                journal.to_lowercase(),
                "wal",
                "{name}: shipped packs must not be WAL (got {journal})"
            );

            let manifest_json: String = conn
                .query_row(
                    "SELECT value FROM manifest WHERE key = 'manifest'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or_else(|e| panic!("{name}: no manifest row: {e}"));
            let manifest: PackManifest = serde_json::from_str(&manifest_json)
                .unwrap_or_else(|e| panic!("{name}: manifest does not parse: {e}"));

            let entries: i64 = conn
                .query_row("SELECT count(*) FROM entries", [], |r| r.get(0))
                .unwrap();
            assert!(entries > 0, "{name}: pack has no entries");

            // Every entry must be reachable by its own lemma, or it is dead weight that
            // inflates the entry count while being unfindable.
            let lemma_terms: i64 = conn
                .query_row(
                    "SELECT count(DISTINCT entry_id) FROM search_terms WHERE term_type = 'lemma'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                lemma_terms, entries,
                "{name}: {entries} entries but only {lemma_terms} are findable by lemma"
            );

            // No orphaned search terms.
            let orphans: i64 = conn
                .query_row(
                    "SELECT count(*) FROM search_terms s \
                     LEFT JOIN entries e ON e.id = s.entry_id WHERE e.id IS NULL",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                orphans, 0,
                "{name}: {orphans} search terms point at no entry"
            );

            // The manifest must describe what the pack actually contains. The old Mandarin
            // pack declared "zh" while every row stored "cmn", so language filtering could
            // never match.
            let mut stmt = conn
                .prepare("SELECT DISTINCT language FROM entries")
                .unwrap();
            let languages: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .filter_map(Result::ok)
                .collect();
            for lang in &languages {
                assert!(
                    manifest.languages.contains(lang),
                    "{name}: entries use language {lang:?} but the manifest declares {:?}",
                    manifest.languages
                );
            }

            // A pack whose language is written in a non-Latin script is useless without a
            // romanized transcription index — that claim shipped with zero such rows.
            if languages.iter().any(|l| l == "zh" || l == "cmn") {
                let transcriptions: i64 = conn
                    .query_row(
                        "SELECT count(*) FROM search_terms WHERE term_type = 'transcription'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert!(
                    transcriptions > 0,
                    "{name}: a Chinese pack needs transcription rows for Pinyin lookup"
                );
            }
        }
    }
}
