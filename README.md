# Golddig

A fast, local-first, open-source dictionary lookup app inspired by the speed and utility of GoldenDict.

Golddig is an independent project and is not affiliated with or endorsed by GoldenDict or GoldenDict-ng.

> **Status:** first vertical-slice implementation is underway. The application code is licensed under MPL-2.0; data packs retain their upstream licenses.

## Product idea

Type or select a word and see the useful evidence together:

- definitions and parts of speech;
- translations;
- American English pronunciation and audio where available;
- examples with translations;
- synonyms and semantic relations;
- common combinations/collocations;
- inflections and alternate forms;
- frequency or commonness;
- visible source and license provenance.

Initial language set: **American English, Catalan, French, German, and Spanish**.

## Principles

1. **Local-first and fast.** Lookup must work offline after data packs are installed.
2. **Text-first results.** The result surface is formatted text—headings, labels, emphasis, lists, examples, and links—not an image-heavy dashboard or a collection of embedded web pages.
3. **One lookup, many sources.** Results appear together without pretending that unrelated senses were perfectly merged.
4. **Open data with provenance.** Every displayed fact remains traceable to its source, snapshot, and license.
5. **Small core, optional packs.** The app stays light; large dictionaries and optional audio are separate downloads.
6. **Interoperable.** Prefer documented, open formats and clean-room importers.
7. **No mandatory account, cloud, telemetry, or AI.** Optional online features must never be required for ordinary lookup.

## Recommended first slice

Build a vertical prototype around a small, reproducible sample of Kaikki/Wiktionary data:

1. stream JSONL into a normalized SQLite database;
2. search exact headwords and inflected forms;
3. show definitions, IPA, examples, translations, synonyms, and source attribution as fast semantic formatted text, with audio as an optional inline action;
4. verify accent-aware and accent-tolerant lookup in all five target languages;
5. measure database size, import time, cold start, and lookup latency before adding more sources.

Then add FreeDict bilingual dictionaries, Tatoeba examples, Open English WordNet, and frequency/collocation data as separate adapters.

## Proposed implementation

- **Desktop shell:** Tauri 2
- **Core/importers:** Rust
- **UI:** Svelte + TypeScript + Vite
- **Storage/search:** SQLite + FTS5, with precomputed language-aware normalized keys
- **Distribution:** tiny application plus independently versioned, immutable data packs

See:

- [`docs/data-sources.md`](docs/data-sources.md)
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/text-first-ui.md`](docs/text-first-ui.md)
- [`docs/goldendict-lessons.md`](docs/goldendict-lessons.md)
- [`docs/roadmap.md`](docs/roadmap.md)
- [`docs/licensing.md`](docs/licensing.md)

## Development

Install the frontend dependencies and run the validation commands:

### Quick Launchers & Shortcut

For quick access on Windows, open the [`launchers/`](launchers/) folder in File Explorer:
- **`launchers/Golddig.lnk`**: Direct Windows shortcut to the native executable with app icon and proper working directory (can be copied to your Desktop or pinned to Start).
- **`launchers/start-golddig.bat`**: Double-click script that launches the desktop app (and auto-builds it if missing).
- **`launchers/start-dev-mode.bat`**: Starts the live development server with hot-reloading.
- **`launchers/run-tests-and-benchmarks.bat`**: Runs the entire test suite and latency benchmark in a console window.

### Command Line

```sh
npm install
npm run check
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo run --manifest-path src-tauri/Cargo.toml --bin golddig-bench
```


### Ingestion CLI (`golddig-pack`)

Golddig provides a command-line tool to build validated SQLite dictionary packs from structured data:

```sh
# Build from authored JSONL entries
cargo run --manifest-path src-tauri/Cargo.toml --bin golddig-pack jsonl fixtures/vertical-slice.manifest.json fixtures/vertical-slice.entries.jsonl packs/my-pack.sqlite

# Build directly from raw Kaikki Wiktextract JSONL dump
cargo run --manifest-path src-tauri/Cargo.toml --bin golddig-pack kaikki fixtures/vertical-slice.manifest.json path/to/kaikki.jsonl packs/kaikki.sqlite [source_id] [max_entries]
```

## Benchmarks

See [`docs/benchmarks/vertical-slice.md`](docs/benchmarks/vertical-slice.md) for latency measurements of the SQLite/Rust engine.

Start the desktop application in development mode with `npm run tauri dev`.

## Important distinction

Wiktionary is the natural primary dictionary source. Wikipedia is an encyclopedia: it can later be an optional article source or a corpus for collocation statistics, but it should not be the foundation of lexical entries.

## Repository policy

Generated databases, downloaded dumps, and audio do not belong in Git. Data packs must be reproducible from pinned upstream snapshots and must carry attribution/license manifests.
