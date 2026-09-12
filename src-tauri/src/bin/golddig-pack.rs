// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Builds and enriches Golddig dictionary packs.
//!
//! An unknown mode used to fall through to a legacy positional form, so a typo silently
//! did something else; and `tatoeba` printed a success message while doing nothing at all.
//! Both now behave.

use anyhow::Result;
use golddig_lib::pack::{build_pack_from_files, build_pack_from_kaikki, merge_tatoeba_examples};
use std::path::PathBuf;

const USAGE: &str = "\
Usage:
  golddig-pack jsonl   <manifest.json> <entries.jsonl> <output.sqlite>
  golddig-pack kaikki  <manifest.json> <kaikki.jsonl> <output.sqlite> [source_id] [max_entries]
  golddig-pack tatoeba <pack.sqlite> <sentences.csv> <links.csv> <lang> [max_per_entry]
  golddig-pack inspect <pack.sqlite>
  golddig-pack lookup  <query> [limit] [packs-dir]

tatoeba takes the raw TSV exports from https://tatoeba.org/downloads and merges example
sentences into an existing pack, matching each sentence against that pack's headwords and
inflected forms. <lang> is a Golddig code (en, ca, es, fr, de, ary, zh).";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");

    match mode {
        "jsonl" => {
            let [manifest, entries, output] = expect(&args, 3, "jsonl")?;
            println!("Building {output:?} from {entries:?}...");
            build_pack_from_files(
                PathBuf::from(manifest).as_path(),
                PathBuf::from(entries).as_path(),
                PathBuf::from(output).as_path(),
            )?;
            println!("Pack built successfully.");
        }

        "kaikki" => {
            let [manifest, entries, output] = expect(&args, 3, "kaikki")?;
            let source_id = args
                .get(5)
                .map(String::as_str)
                .unwrap_or("kaikki-wiktionary");
            let max_entries = args.get(6).and_then(|s| s.parse::<usize>().ok());
            println!("Building {output:?} from Kaikki dump {entries:?}...");
            let count = build_pack_from_kaikki(
                PathBuf::from(manifest).as_path(),
                PathBuf::from(entries).as_path(),
                PathBuf::from(output).as_path(),
                source_id,
                max_entries,
            )?;
            println!("Successfully built pack with {count} entries.");
        }

        "tatoeba" => {
            let [pack, sentences, links] = expect(&args, 3, "tatoeba")?;
            let lang = args
                .get(5)
                .map(String::as_str)
                .ok_or_else(|| anyhow::anyhow!("tatoeba needs a language code\n\n{USAGE}"))?;
            let max_per_entry = args
                .get(6)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(3);
            let added = merge_tatoeba_examples(
                PathBuf::from(&pack).as_path(),
                PathBuf::from(&sentences).as_path(),
                PathBuf::from(&links).as_path(),
                lang,
                max_per_entry,
            )?;
            println!("Merged {added} Tatoeba examples into {pack}.");
        }

        "inspect" => {
            let pack = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("inspect needs a pack path\n\n{USAGE}"))?;
            let conn = rusqlite::Connection::open_with_flags(
                pack,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            let entries: i64 = conn.query_row("SELECT count(*) FROM entries", [], |r| r.get(0))?;
            let manifest: String = conn.query_row(
                "SELECT value FROM manifest WHERE key = 'manifest'",
                [],
                |r| r.get(0),
            )?;
            println!("{pack}: {entries} entries\n{manifest}");
            let mut stmt =
                conn.prepare("SELECT term_type, count(*) FROM search_terms GROUP BY 1")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
            for row in rows {
                let (kind, n) = row?;
                println!("  {kind}: {n}");
            }
        }

        // Runs the real engine over the real packs and prints the ranked result. Exists so
        // ranking can be checked against shipped artifacts instead of only against the
        // synthetic packs in the test suite — searching `man` used to return Arabic
        // inflected forms above the English headword, and nothing surfaced that.
        "lookup" => {
            let query = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("lookup needs a query\n\n{USAGE}"))?;
            let limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);
            let dir = args.get(4).map(String::as_str).unwrap_or("packs");

            let mut engine = golddig_lib::search::SearchEngine::new();
            let loaded = engine.load_directory(dir)?;
            for err in engine.load_errors() {
                eprintln!("warning: {} — {}", err.path, err.message);
            }
            println!("{loaded} pack(s) from {dir}/");

            let started = std::time::Instant::now();
            let hits = engine.suggest(query, limit)?;
            let elapsed = started.elapsed();
            println!("{} hit(s) for {query:?} in {elapsed:?}\n", hits.len());

            for (i, hit) in hits.iter().enumerate() {
                println!(
                    "{:>2}. {:<24} {:<5} {:<10} matched {:?} as {}",
                    i + 1,
                    hit.lemma,
                    hit.language,
                    hit.pos.as_deref().unwrap_or("-"),
                    hit.matched_term,
                    hit.match_type
                );
            }
        }

        // Anything else is an error. Falling through to a legacy positional form meant a
        // mistyped mode quietly ran a different command.
        other => {
            if !other.is_empty() {
                eprintln!("Unknown mode: {other}\n");
            }
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }

    Ok(())
}

/// Pulls exactly `n` positional arguments following the mode, or prints usage.
fn expect(args: &[String], n: usize, mode: &str) -> Result<[String; 3]> {
    if args.len() < 2 + n {
        anyhow::bail!("{mode} needs {n} arguments\n\n{USAGE}");
    }
    Ok([args[2].clone(), args[3].clone(), args[4].clone()])
}
