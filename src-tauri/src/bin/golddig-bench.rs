use golddig_lib::search::SearchEngine;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let pack_path = PathBuf::from("packs/vertical-slice.sqlite");
    if !pack_path.exists() {
        eprintln!("Pack file not found at {:?}", pack_path);
        std::process::exit(1);
    }

    let start_open = Instant::now();
    let engine = SearchEngine::open_pack(&pack_path).expect("Failed to open pack");
    let open_duration = start_open.elapsed();

    println!("=== Golddig Vertical Slice Benchmark ===");
    println!("Database pack: {:?}", pack_path);
    println!("Pack open time: {:?}", open_duration);

    let queries = vec![
        ("exact english", "light"),
        ("inflected english", "lights"),
        ("catalan ela geminada", "col·lecció"),
        ("spanish collision 1", "ano"),
        ("spanish collision 2", "año"),
        ("german eszett", "Straße"),
        ("french ligature", "cœur"),
        ("prefix search", "li"),
    ];

    println!("\nLatency measurements (1000 iterations per query):");
    for (label, q) in queries {
        let start = Instant::now();
        let iterations = 1000;
        let mut total_results = 0;
        for _ in 0..iterations {
            let res = engine.suggest(q, 10).unwrap();
            total_results += res.len();
        }
        let elapsed = start.elapsed();
        let avg_micros = elapsed.as_micros() as f64 / iterations as f64;
        println!(
            "  Query '{:22}' (term: {:10}): avg {:6.2} µs ({:.3} ms) [total matches: {}]",
            label,
            q,
            avg_micros,
            avg_micros / 1000.0,
            total_results / iterations
        );
    }

    println!("\nBenchmark complete.");
}
