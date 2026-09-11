use anyhow::Result;
use golddig_lib::pack::build_pack_from_files;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: golddig-pack <manifest.json> <entries.jsonl> <output.sqlite>");
        std::process::exit(1);
    }

    let manifest = PathBuf::from(&args[1]);
    let entries = PathBuf::from(&args[2]);
    let output = PathBuf::from(&args[3]);

    println!(
        "Building pack from {:?} and {:?} to {:?}...",
        manifest, entries, output
    );
    build_pack_from_files(&manifest, &entries, &output)?;
    println!("Pack built successfully!");

    Ok(())
}
