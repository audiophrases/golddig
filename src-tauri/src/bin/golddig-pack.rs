use anyhow::Result;
use golddig_lib::pack::{build_pack_from_files, build_pack_from_kaikki};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage:");
        eprintln!("  golddig-pack jsonl <manifest.json> <entries.jsonl> <output.sqlite>");
        eprintln!("  golddig-pack kaikki <manifest.json> <kaikki.jsonl> <output.sqlite> [source_id] [max_entries]");
        eprintln!("  golddig-pack tatoeba <manifest.json> <tatoeba.json> <output.sqlite>");
        std::process::exit(1);
    }

    let mode = &args[1];
    if mode == "kaikki" {
        let manifest = PathBuf::from(&args[2]);
        let entries = PathBuf::from(&args[3]);
        let output = PathBuf::from(&args[4]);
        let source_id = args
            .get(5)
            .map(|s| s.as_str())
            .unwrap_or("kaikki-wiktionary");
        let max_entries = args.get(6).and_then(|s| s.parse::<usize>().ok());

        println!(
            "Building pack from Kaikki Wiktextract {:?} and {:?} to {:?}...",
            manifest, entries, output
        );
        let count = build_pack_from_kaikki(&manifest, &entries, &output, source_id, max_entries)?;
        println!("Successfully built pack with {} entries!", count);
    } else if mode == "tatoeba" {
        println!("Tatoeba sentence pair pack ingestion mode ready.");
    } else {
        let manifest = PathBuf::from(&args[2]);
        let entries = PathBuf::from(&args[3]);
        let output = PathBuf::from(&args[4]);

        println!(
            "Building pack from {:?} and {:?} to {:?}...",
            manifest, entries, output
        );
        build_pack_from_files(&manifest, &entries, &output)?;
        println!("Pack built successfully!");
    }

    Ok(())
}
