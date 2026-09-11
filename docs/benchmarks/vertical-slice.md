# Golddig Vertical Slice Benchmark Report

Date: 2026-09-11
Host: Windows 11 (x86_64-pc-windows-msvc)
Toolchain: Rust 1.98.1, Node v24.15.0, SQLite via rusqlite 0.34

## Pack & Storage Metrics
- **Fixture Pack:** `packs/vertical-slice.sqlite`
- **Pack Size:** ~24 KB
- **Pack Open Latency:** ~560 µs (0.56 ms)

## Query Latency Benchmarks (1,000 iterations per query)
| Query Type | Term | Language / Feature | Average Latency |
|---|---|---|---|
| Exact lookup | `light` | en-US exact lemma | **118.75 µs** (0.119 ms) |
| Inflected form | `lights` | en-US plural form | **110.24 µs** (0.110 ms) |
| Ela geminada | `col·lecció` | Catalan `l·l` special ligature | **103.31 µs** (0.103 ms) |
| Diacritic collision 1 | `ano` | Spanish `ano` (never conflated with `año`) | **127.83 µs** (0.128 ms) |
| Diacritic collision 2 | `año` | Spanish `año` (never conflated with `ano`) | **128.81 µs** (0.129 ms) |
| Eszett preservation | `Straße` | German `ß` | **125.25 µs** (0.125 ms) |
| Ligature preservation | `cœur` | French `œ` | **106.20 µs** (0.106 ms) |
| Prefix lookup | `li` | Multi-language prefix suggestions | **123.07 µs** (0.123 ms) |

## Performance Target Assessment
- **Prefix suggestions target:** < 50 ms. **Observed:** ~0.12 ms (exceeds budget by >400x).
- **Exact lookup target:** < 100 ms. **Observed:** ~0.12 ms (exceeds budget by >800x).
- **Zero network requests:** All queries execute against the local embedded SQLite pack.
