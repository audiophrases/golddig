# Lookup latency

Date: 2026-09-11
Host: Windows 11 (x86_64-pc-windows-msvc), Rust 1.98.1, rusqlite with bundled SQLite
Measured with `cargo run --release --bin golddig-bench -- <packs>`

Every figure below names the pack it was measured on, with that pack's size and entry count.
The previous version of this document reported ~0.10–0.13 ms as "the engine's performance"
without disclosing that every measurement came from a 7-entry, 57 KB authored fixture. Those
numbers were real; they just did not describe a dictionary.

`n` is the sample count. Indexed queries get 200 samples; the two known-unindexed shapes get
5, because 200 would take minutes.

## The authored test fixture — 10 entries, 0.1 MB

Useful only as a floor. This is what the old report measured.

| Query | Shape | mean | p50 | p95 |
| --- | --- | --- | --- | --- |
| `light` | exact lemma | 0.17 ms | 0.15 ms | 0.25 ms |
| `lig` | 3-char prefix | 0.18 ms | 0.16 ms | 0.23 ms |
| `l?ght` | wildcard | 0.18 ms | 0.13 ms | 0.20 ms |
| `l*t` | wildcard | 0.14 ms | 0.13 ms | 0.19 ms |
| `morning light` | phrase (content scan) | 0.31 ms | 0.31 ms | 0.37 ms |

## Moroccan Arabic (Darija) — 2,342 entries, 5.8 MB

| Query | Shape | mean | p50 | p95 |
| --- | --- | --- | --- | --- |
| `h` | 1-char prefix | 0.35 ms | 0.26 ms | 0.43 ms |
| `lig` | 3-char prefix | 0.18 ms | 0.16 ms | 0.23 ms |
| `light` | exact lemma | 0.17 ms | 0.15 ms | 0.25 ms |
| `morning light` | phrase (content scan) | 10.0 ms | 9.6 ms | 11.8 ms |

## Catalan — 198,460 entries, 261 MB

The realistic case, and the one that matters.

| Query | Shape | mean | p50 | p95 |
| --- | --- | --- | --- | --- |
| `h` | 1-char prefix | 0.37 ms | 0.31 ms | 0.64 ms |
| `lig` | 3-char prefix | 0.21 ms | 0.19 ms | 0.35 ms |
| `light` | exact lemma | 0.20 ms | 0.16 ms | 0.33 ms |
| `l?ght` | wildcard, literal prefix | 0.80 ms | 0.63 ms | 1.21 ms |
| `l*t` | wildcard, literal prefix | 0.35 ms | 0.28 ms | 0.51 ms |
| `*ight` | wildcard, literal **suffix** | 0.13 ms | 0.12 ms | 0.19 ms |
| `morning light` | phrase (content scan) | **812 ms** | 802 ms | 841 ms |

Pack open: 6.0 ms.

## Three packs together — 200,812 entries

What the app actually runs, with results merged and globally ranked across packs.

| Query | mean | p50 | p95 |
| --- | --- | --- | --- |
| `h` (1-char prefix) | 0.77 ms | 0.74 ms | 0.89 ms |
| `lig` (3-char prefix) | 0.53 ms | 0.51 ms | 0.60 ms |
| `light` (exact lemma) | 0.93 ms | 0.74 ms | 1.27 ms |
| `man` (cross-pack collision) | 0.73 ms | 0.62 ms | 0.91 ms |
| `l*t` (wildcard) | 0.55 ms | 0.51 ms | 0.72 ms |

Against the stated budgets — prefix suggestions under 50 ms, first useful text under 100 ms —
every indexed path has ~50× headroom on a 200k-entry library.

## What the two index fixes bought

Both were measured, not estimated.

**Prefix probes: ~20–34 ms → ~0.2–0.4 ms per keystroke.** `suggest` matched with
`term LIKE 'lig%'`. SQLite's `LIKE` is case-insensitive by default and therefore cannot use a
`BINARY`-collated index, so `EXPLAIN QUERY PLAN` reported `SCAN` and every keystroke walked
the entire term table — 19–34 ms on a 61k-entry pack, growing linearly. Replaced with a
half-open range probe (`term >= 'lig' AND term < 'lih'`) over a covering index, which is
index-usable unconditionally. The current figure is *faster on a pack three times larger*.

**Suffix patterns: 505 ms → 0.13 ms.** No forward b-tree can serve `*ight`; on Catalan it
cost 505 ms mean and 545 ms p95. `search_terms` now also stores `term_rev`, the term with its
characters reversed, so a literal suffix becomes a prefix probe on the reversed index
(`SEARCH s USING INDEX idx_search_term_rev`), confirmed with `GLOB` so the match stays exact.
A ~3,750× improvement. Packs built before this column exists still open and fall back to the
forward scan — `PackHandle::has_term_rev` detects it.

## The one slow path left

**Phrase content search: 812 ms on a 198k-entry pack.** When a multi-word query is not
satisfied by the indexed tiers, Golddig scans `entries.data_json` with `LIKE` to search
definitions and examples. That is an unindexed scan of hundreds of megabytes of JSON and no
b-tree can help.

Three things bound the damage: it runs only for queries containing a space, only after the
indexed tiers failed to fill the result list, and it stops as soon as the list is full rather
than scanning every pack. It is never on the single-word keystroke path.

The real fix is a contentless FTS5 index over definition and example text, which would turn
this into a fast ranked tier instead of a slow fallback. It is the first item in
[`../roadmap.md`](../roadmap.md).

## Reproducing

```sh
python scripts/build_all_packs.py ary      # ~6 MB, quickest real pack
cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-bench
cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-bench -- --fixture
```

With no argument the benchmark measures every pack in `packs/`, then all of them together.
