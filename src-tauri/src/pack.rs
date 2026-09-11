use crate::model::{EntryRecord, PackManifest};
use crate::normalization::generate_search_keys;
use rusqlite::{params, Connection, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

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
            term_type TEXT NOT NULL, -- 'lemma', 'form', 'loose'
            loose_key TEXT NOT NULL,
            language TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_search_term ON search_terms(term);
        CREATE INDEX IF NOT EXISTS idx_search_loose ON search_terms(loose_key);
        CREATE INDEX IF NOT EXISTS idx_entries_lang ON entries(language);
        "#,
    )?;
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

        // Search terms: Lemma
        let lemma_keys = generate_search_keys(&entry.lemma, &entry.language);
        tx.execute(
            "INSERT INTO search_terms (entry_id, term, term_type, loose_key, language) VALUES (?, ?, 'lemma', ?, ?)",
            params![entry.id, lemma_keys.exact_normalized, lemma_keys.loose_key, entry.language],
        )?;

        // Search terms: Forms
        for f in &entry.forms {
            let form_keys = generate_search_keys(&f.form, &entry.language);
            tx.execute(
                "INSERT INTO search_terms (entry_id, term, term_type, loose_key, language) VALUES (?, ?, 'form', ?, ?)",
                params![entry.id, form_keys.exact_normalized, form_keys.loose_key, entry.language],
            )?;
        }

        // Search terms: Transcriptions / Pinyin
        for p in &entry.pronunciations {
            if p.kind == "pinyin" {
                let pinyin_keys = generate_search_keys(&p.ipa, &entry.language);
                tx.execute(
                    "INSERT INTO search_terms (entry_id, term, term_type, loose_key, language) VALUES (?, ?, 'transcription', ?, ?)",
                    params![entry.id, pinyin_keys.exact_normalized, pinyin_keys.loose_key, entry.language],
                )?;
            }
        }
    }

    tx.commit()?;

    // Quick verification
    let quick_check: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))?;
    if quick_check != "ok" {
        anyhow::bail!("Pack SQLite corruption check failed: {}", quick_check);
    }

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

    let mut count = 0;
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let raw_entry = match serde_json::from_str::<crate::importers::kaikki::KaikkiEntry>(trimmed)
        {
            Ok(e) => e,
            Err(_) => continue,
        };

        if let Some(entry) = raw_entry.into_entry_record(source_id) {
            let data_json = serde_json::to_string(&entry)?;

            tx.execute(
                "INSERT OR REPLACE INTO entries (id, language, lemma, pos, data_json) VALUES (?, ?, ?, ?, ?)",
                params![entry.id, entry.language, entry.lemma, entry.pos, data_json],
            )?;

            let lemma_keys = generate_search_keys(&entry.lemma, &entry.language);
            tx.execute(
                "INSERT INTO search_terms (entry_id, term, term_type, loose_key, language) VALUES (?, ?, 'lemma', ?, ?)",
                params![entry.id, lemma_keys.exact_normalized, lemma_keys.loose_key, entry.language],
            )?;

            for f in &entry.forms {
                let form_keys = generate_search_keys(&f.form, &entry.language);
                tx.execute(
                    "INSERT INTO search_terms (entry_id, term, term_type, loose_key, language) VALUES (?, ?, 'form', ?, ?)",
                    params![entry.id, form_keys.exact_normalized, form_keys.loose_key, entry.language],
                )?;
            }

            // Also index pinyin / romanized transcriptions in search_terms so users can search by transcription
            for p in &entry.pronunciations {
                if p.kind == "pinyin" {
                    let pinyin_keys = generate_search_keys(&p.ipa, &entry.language);
                    tx.execute(
                        "INSERT INTO search_terms (entry_id, term, term_type, loose_key, language) VALUES (?, ?, 'transcription', ?, ?)",
                        params![entry.id, pinyin_keys.exact_normalized, pinyin_keys.loose_key, entry.language],
                    )?;
                }
            }

            count += 1;
            if let Some(max) = max_entries {
                if count >= max {
                    break;
                }
            }
        }
    }

    tx.commit()?;

    let quick_check: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))?;
    if quick_check != "ok" {
        anyhow::bail!("Pack SQLite corruption check failed: {}", quick_check);
    }

    Ok(count)
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
}
