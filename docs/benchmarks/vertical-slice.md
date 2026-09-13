# Lookup latency

Date: 2026-09-12 (re-measured after the FTS5 index landed)
Host: Windows 11 (x86_64-pc-windows-msvc), Rust 1.98.1, rusqlite with bundled SQLite
Measured with `cargo run --release --bin golddig-bench -- <packs>`

Every figure below names the pack it was measured on, with that pack's size and entry count.
The previous version of this document reported ~0.10–0.13 ms as "the engine's performance"
without disclosing that every measurement came from a 7-entry, 57 KB authored fixture. Those
numbers were real; they just did not describe a dictionary.

`n` is the sample count: 200 for the fast shapes, 5 for the two that were historically slow
(a leading wildcard and a phrase). Both are now index-backed, so their small `n` is a leftover
of when 200 samples would have taken minutes.

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
| `morning light` | phrase (budgeted scan) | 14.0 ms | 12.1 ms | 22.3 ms |

## Catalan — 198,460 entries, 261 MB

The realistic case, and the one that matters.

| Query | Shape | mean | p50 | p95 |
| --- | --- | --- | --- | --- |
| `h` | 1-char prefix | 0.37 ms | 0.31 ms | 0.64 ms |
| `lig` | 3-char prefix | 0.21 ms | 0.19 ms | 0.35 ms |
| `light` | exact lemma | 0.20 ms | 0.16 ms | 0.33 ms |
| `l?ght` | wildcard, literal prefix | 0.80 ms | 0.63 ms | 1.21 ms |
| `l*t` | wildcard, literal prefix | 0.35 ms | 0.28 ms | 0.51 ms |
| `*ight` | wildcard, literal **suffix** | 0.13 ms | 0.12 ms | 0.18 ms |
| `morning light` | phrase (FTS5) | 0.36 ms | 0.29 ms | 0.55 ms |

Pack open: 2.7 ms. Pack size 199.7 MB after the lean format (was 273.8 MB); the FTS5 index is
about 13 MB of that.

## English — 1,491,592 entries, 1,986 MB

The largest pack, built from the full English Wiktionary extract. This is where the
unindexed shapes stop being merely slow.

| Query | Shape | mean | p50 | p95 |
| --- | --- | --- | --- | --- |
| `h` | 1-char prefix | 0.49 ms | 0.37 ms | 0.50 ms |
| `lig` | 3-char prefix | 0.46 ms | 0.30 ms | 0.32 ms |
| `light` | exact lemma | 0.39 ms | 0.32 ms | 0.34 ms |
| `l?ght` | wildcard, 1-char literal prefix | 6.1 ms | 6.0 ms | 7.2 ms |
| `l*t` | wildcard, literal prefix | 0.42 ms | 0.40 ms | 0.50 ms |
| `*ight` | wildcard, literal suffix | 0.35 ms | 0.24 ms | 0.79 ms |
| `morning light` | phrase (FTS5) | **1.2 ms** | 1.1 ms | 1.8 ms |

Pack open: 9.3 ms. Pack size 1,674.0 MB after the lean format (was 2,190.7 MB). Every path
except `l?ght` stays near a millisecond on 1.5M entries, and the phrase query returns a full
10 hits rather than a truncated few.

`l?ght` is the weakest indexed case: its literal prefix is one character, so the range probe
covers every term beginning with `l` and GLOB filters them. 6 ms is still well inside budget.

## All eight packs together — 3,471,857 entries

What the app actually runs: en, ca, es, fr, de, ary, zh and the fixture, merged and globally
ranked, with ties broken by the reader's pack order.

| Query | mean | p50 | p95 |
| --- | --- | --- | --- |
| `h` (1-char prefix) | 3.3 ms | 3.1 ms | 4.2 ms |
| `lig` (3-char prefix) | 3.6 ms | 3.5 ms | 4.0 ms |
| `light` (exact lemma) | 2.5 ms | 2.4 ms | 2.7 ms |
| `man` (exact lemma in six languages at once) | 7.0 ms | 6.4 ms | 8.5 ms |
| `l*t` (wildcard) | 7.9 ms | 7.7 ms | 10.0 ms |

Against the stated budgets — prefix suggestions under 50 ms, first useful text under 100 ms —
every path has at least 6× headroom across 3.5M entries in eight dictionaries. Cost grows with
the number of enabled packs, since each contributes candidates to the global ranking.

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

## What the full-text index bought

**Phrase search: 812 ms → 0.36 ms on Catalan, 10.5 s → 1.2 ms on English.**

Phrase lookup used to scan `entries.data_json` with `LIKE` — on the English pack that is
roughly 2 GB of JSON, measured at 10.5 s mean and 18 s p95, reachable by typing any two words.
A 120 ms-per-pack wall-clock budget made it predictable at ~247 ms but cost completeness: the
same query returned 1 hit instead of 10.

`entries_fts` replaces it: a contentless FTS5 index over headwords, definitions, examples and
related terms, ranked with `bm25(entries_fts, 20, 10, 3, 1)` so a headword hit outranks a
definition, a definition outranks an example, and an example outranks a related term.
Contentless means only the inverted index is stored, never a second copy of the text, which is
why it costs about +5% pack size.

Two details matter. Including `headwords` in the index let the unindexed
`LIKE '%…%'` over `search_terms` be deleted outright — with FTS5 present but headwords absent,
that tier still burned its full 120 ms budget on every multi-word query whether it matched or
not, which is why phrase search sat at 124 ms until headwords were added. And the tokenizer is
configured `remove_diacritics 0` on purpose: letting it fold would conflate Spanish ñ with n.
Diacritic tolerance stays an explicit, tested rule in `normalization.rs`.

The fallback is kept for packs built before the index existed: `PackHandle::has_fts` detects
its absence and routes those packs to the old budgeted scan, so a format addition never makes
an installed dictionary unreadable.

## The one slow path left

**`l?ght` — a wildcard whose literal prefix is one character — is 6.1 ms on the English pack.**
The range probe covers every term beginning with `l` and GLOB filters the rest. It is the
weakest indexed shape and still well inside budget, so it is not worth more machinery.

## Pack size

Measured after removing two genuine redundancies from the format: the entry's text id stored
five times over in `search_terms` and its indexes, and the pack-default `source_id` repeated
on every sense, example, form and pronunciation (12.6% of the JSON). An unused `language`
column and a redundant `UNIQUE` index went with them. Term counts and lookup latency are
identical before and after.

| Pack | Entries | Before | After | Change | Download (gzip) |
| --- | ---: | ---: | ---: | ---: | ---: |
| English | 1,491,592 | 2,190.7 MB | 1,674.0 MB | −23.6% | 500 MB (3.3×) |
| Chinese | 194,134 | 1,604.0 MB | 1,149.3 MB | −28.4% | 257 MB (4.5×) |
| German | 371,241 | 1,402.6 MB | 1,000.6 MB | −28.7% | 169 MB (5.9×) |
| Spanish | 810,914 | 1,216.4 MB | 904.8 MB | −25.6% | 180 MB (5.0×) |
| French | 403,164 | 590.6 MB | 444.2 MB | −24.8% | 89 MB (5.0×) |
| Catalan | 198,460 | 273.8 MB | 199.7 MB | −27.1% | 42 MB (4.8×) |
| Darija | 2,342 | 6.8 MB | 4.4 MB | −35.3% | 1 MB (3.8×) |
| **Total** | **3,471,857** | **7,284.9 MB** | **5,376.9 MB** | **−26.2%** | **1,238 MB** |

English compresses worst. Its content is the richest — 155,873 entries carry translation
tables — and richer content is less repetitive than the inflection-heavy German and Spanish
packs, which compress best.

Per-row compression of the JSON payload was measured and deliberately not adopted: with a
shared dictionary it reaches 7.8× on the payload for a 5 µs per-entry decode, which would
take another ~40% off the installed size — but an already-compressed pack barely gzips again,
so the download would grow slightly. This project optimises for download; the numbers are
recorded here in case that changes.

## Reproducing

```sh
python scripts/build_all_packs.py ary      # ~6 MB, quickest real pack
cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-bench
cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-bench -- --fixture
```

With no argument the benchmark measures every pack in `packs/`, then all of them together.
