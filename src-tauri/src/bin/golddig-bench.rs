// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Latency benchmark for the Golddig lookup path.
//!
//! Takes the pack(s) to measure as arguments, because the previous version hardcoded
//! `packs/vertical-slice.sqlite` — a 7-entry, 57 KB authored fixture — and its numbers
//! (0.10–0.13 ms) were published in docs/benchmarks as the engine's performance. Real
//! packs are two to three orders of magnitude larger.
//!
//!     golddig-bench                          # every pack in packs/
//!     golddig-bench packs/kaikki-english.sqlite
//!     golddig-bench --fixture                # just the authored fixture
//!
//! Reports median and p95 alongside the mean: a mean alone hides the tail that actually
//! determines whether typing feels instant.

use golddig_lib::search::SearchEngine;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Indexed queries are cheap, so measure many. Unindexed ones (a leading wildcard, the
/// content-scan fallback) cost hundreds of milliseconds on a large pack, so a few samples
/// are enough — and 200 of them would take minutes per query.
const FAST_ITERATIONS: usize = 200;
const SLOW_ITERATIONS: usize = 5;

fn percentile(sorted_micros: &[f64], pct: f64) -> f64 {
    if sorted_micros.is_empty() {
        return 0.0;
    }
    let idx = ((sorted_micros.len() as f64 - 1.0) * pct).round() as usize;
    sorted_micros[idx]
}

fn bench_n(engine: &SearchEngine, label: &str, query: &str, iterations: usize) {
    let mut samples = Vec::with_capacity(iterations);
    let mut matches = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        let results = engine.suggest(query, 10).unwrap_or_default();
        samples.push(start.elapsed().as_micros() as f64);
        matches = results.len();
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    println!(
        "  {:22} {:16} n={:<4} mean {:10.1} us  p50 {:10.1}  p95 {:10.1}  hits {}",
        label,
        query,
        iterations,
        mean,
        percentile(&samples, 0.50),
        percentile(&samples, 0.95),
        matches
    );
}

/// Indexed query: many samples.
fn bench(engine: &SearchEngine, label: &str, query: &str) {
    bench_n(engine, label, query, FAST_ITERATIONS);
}

/// Known-unindexed query: few samples.
fn bench_slow(engine: &SearchEngine, label: &str, query: &str) {
    bench_n(engine, label, query, SLOW_ITERATIONS);
}

fn discover_packs(dir: &Path) -> Vec<PathBuf> {
    let mut packs: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("sqlite"))
        .collect();
    packs.sort();
    packs
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let packs: Vec<PathBuf> = if args.iter().any(|a| a == "--fixture") {
        vec![PathBuf::from("packs/vertical-slice.sqlite")]
    } else if args.is_empty() {
        discover_packs(Path::new("packs"))
    } else {
        args.iter().map(PathBuf::from).collect()
    };

    if packs.is_empty() {
        eprintln!("No packs found. Build one with: python scripts/build_all_packs.py ary");
        std::process::exit(1);
    }

    // Per-pack open cost and single-pack latency.
    for pack in &packs {
        if !pack.exists() {
            eprintln!("skipping missing pack {}", pack.display());
            continue;
        }
        let size_mb = std::fs::metadata(pack).map(|m| m.len()).unwrap_or(0) as f64 / 1_048_576.0;
        let start = Instant::now();
        let engine = match SearchEngine::open_pack(pack) {
            Ok(e) => e,
            Err(err) => {
                eprintln!("could not open {}: {err}", pack.display());
                continue;
            }
        };
        let open = start.elapsed();
        let entries = engine
            .list_packs()
            .first()
            .map(|p| p.entry_count)
            .unwrap_or(0);

        println!(
            "\n=== {}  ({:.1} MB, {} entries)  open {:?}",
            pack.display(),
            size_mb,
            entries,
            open
        );
        bench(&engine, "single char prefix", "h");
        bench(&engine, "three char prefix", "lig");
        bench(&engine, "exact lemma", "light");
        bench(&engine, "wildcard single", "l?ght");
        bench(&engine, "wildcard multi", "l*t");
        bench_slow(&engine, "leading wildcard", "*ight");
        bench_slow(&engine, "phrase", "morning light");
    }

    // All packs together — what the app actually runs.
    if packs.len() > 1 {
        let mut engine = SearchEngine::new();
        for pack in &packs {
            let _ = engine.add_pack(pack, true);
        }
        let loaded = engine.list_packs();
        let total: usize = loaded.iter().map(|p| p.entry_count).sum();
        println!(
            "\n=== all {} packs together ({} entries) — the shipped configuration",
            loaded.len(),
            total
        );
        bench(&engine, "single char prefix", "h");
        bench(&engine, "three char prefix", "lig");
        bench(&engine, "exact lemma", "light");
        bench(&engine, "cross-pack collision", "man");
        bench(&engine, "wildcard multi", "l*t");
    }

    println!();
}
