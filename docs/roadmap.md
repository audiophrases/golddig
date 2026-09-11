# Initial roadmap

## Phase 0 — decisions and legal/data groundwork

- Confirm application license (recommended candidates: GPL-3.0-or-later or MPL-2.0).
- Confirm Tauri/Rust/Svelte architecture.
- Define the pack manifest and attribution display.
- Pin one small, legally redistributable fixture for each target language.
- Write a data acceptance checklist: license, provenance, schema coverage, update cadence, quality, and size.

**Exit:** no data enters a distributable pack without a recorded license and reproducible source.

## Phase 1 — vertical lookup slice

- Scaffold Tauri 2, Rust, Svelte, TypeScript, and tests.
- Bundle SQLite with FTS5 and access it through the Rust core (`rusqlite`), not by exposing SQL to the frontend.
- Implement the common lexical model and pack schema.
- Implement streaming Kaikki JSONL import.
- Index lemmas and forms in SQLite.
- Build exact, normalized, prefix, and typo-fallback lookup.
- Render definitions, IPA, forms, examples, translations, synonyms, labels, and attribution through the text-first semantic component set; audio is an optional inline action.
- Keep source CSS, JavaScript, images, and remote media out of the initial result surface.
- Benchmark startup, lookup, import memory/time, pack size, and time-to-visible-text with normal and pathological entries.

**Exit:** a real five-language sample pack can be built from scratch and searched through the desktop UI.

## Phase 2 — useful bilingual dictionary

- Add FreeDict TEI adapter.
- Add Tatoeba sentence/translation adapter, initially preferring CC0 records or preserving CC BY attribution.
- Add Open English WordNet and compatible multilingual wordnets.
- Add `wordfreq`-derived frequency/commonness with its required attribution/share-alike handling.
- Add signed `.gdpkg` download/update UI with staging, hashes, integrity checks, side-by-side versions, atomic activation, and rollback.

**Exit:** common words produce translations, examples, synonyms, and frequency alongside definitions.

## Phase 3 — “commonly used with”

- Evaluate Leipzig co-occurrence downloads and carefully verify each corpus license.
- Build lemmatized collocations by language and part of speech.
- Store score, corpus, sample count, date, and relation type.
- Distinguish statistical collocations from editorial Wiktionary related/derived terms.

**Exit:** collocations are useful, source-labeled, and reproducibly generated for all five languages.

## Phase 4 — GoldenDict-style interoperability

- Import StarDict with companion resources and 64-bit-safe offsets.
- Add ABBYY DSL with a documented supported-tag subset.
- Use Dictd as a simple compatibility proving ground.
- Add unencrypted MDict only after safe rich-article rendering is proven.
- Add folder watching, persistent fingerprints, explicit rescans, and non-mutating reference-in-place imports.
- Add a global-shortcut popup and history. Keep clipboard monitoring opt-in, visibly indicated, and history-free.
- Evaluate XDXF, ZIM, Slob, and other formats based on open documentation and measured user demand.
- Keep user-supplied dictionaries local; do not redistribute unknown/proprietary content.

**Exit:** Golddig is a practical daily replacement for fast multi-dictionary lookup.

## First implementation tasks

1. Install Rust with the official `rustup` installer and verify the Windows Tauri prerequisites.
2. Scaffold the desktop app and test runner.
3. Freeze a versioned `pack-manifest.schema.json`.
4. Create a tiny authored/redistributable multilingual fixture.
5. Write failing importer and normalization tests.
6. Implement Kaikki import into SQLite.
7. Build the first lookup screen.
8. Measure before expanding datasets.
