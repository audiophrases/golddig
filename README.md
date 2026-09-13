# Golddig

A fast, local-first, open-source dictionary lookup app inspired by the speed and utility of GoldenDict.

Golddig is an independent project and is not affiliated with or endorsed by GoldenDict or GoldenDict-ng.

> **Status:** working desktop application with a real multi-language pack pipeline. The
> application code is licensed under MPL-2.0; data packs retain their upstream licenses —
> see [`NOTICE.md`](NOTICE.md).

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
- **Storage/search:** SQLite. Each pack stores entries as a JSON payload, a `search_terms`
  index of lemmas, inflected forms and romanized transcriptions — each with a precomputed
  language-aware loose key and a reversed form for suffix patterns, keyed to the entry by
  integer rowid — and a contentless FTS5 index over headwords, definitions, examples and
  related terms. Headword lookup uses covering b-tree indexes and half-open range probes;
  phrase lookup uses FTS5 ranked by bm25. Provenance equal to the pack default is omitted
  from stored JSON and restored on read.
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

### Search Features

- **Prefix and exact match:** index-backed. Prefix queries use a half-open range
  (`term >= 'lig' AND term < 'lih'`) instead of `LIKE 'lig%'`, because SQLite cannot use a
  BINARY index for its default case-insensitive `LIKE` — with `LIKE`, every keystroke
  full-scanned the whole term table.
- **Wildcard Search (`?` and `*`):**
  - `?` replaces exactly 1 character (e.g. `l?ght` matches `light`).
  - `*` replaces 0 or more characters (e.g. `l*t` matches `light`).
- **Phrase lookup:** multi-word queries (`morning light`) search headwords, sense
  definitions, example sentences and related terms through a contentless FTS5 index, ranked by
  bm25 with headwords weighted above definitions, definitions above examples. 0.33 ms on a
  198k-entry pack. Query text is passed as a quoted FTS5 phrase, so `AND`, `*`, `^`, `:` and
  friends stay literal rather than changing the search. Packs built before the index existed
  fall back to a `LIKE` scan under a 120 ms-per-pack budget.
- **Ranking:** candidates from every enabled pack are merged and sorted *globally* by match
  tier (exact lemma, exact transcription, exact form, then prefix, then loose, then full-text),
  then deduplicated by entry. One entry never appears twice in a result list. Within a tier,
  the headword spelled as typed wins, then ordinary words over proper nouns and affixes, and
  finally **your pack order** — reorder packs with the arrows in the Packs panel. `man` is an
  exact lemma in five languages at once, so which one leads is your choice, not a guess.
- **Accent & Diacritic Normalization:**
  - Catalan ela geminada (`col·lecció` / `col.leccio`), Spanish `ñ` vs `n`, German `ß` vs `ss`, French `œ` vs `oe`, Arabic Alif/Tashkeel, and Pinyin tones.

### Pronunciation

Two buttons next to the headword, both user-initiated and both using the network — lookup
itself never does.

- **Speaker** synthesizes the headword (or an example sentence) with a Microsoft neural
  voice: `ca-ES-Joana`, `es-ES-Alvaro`, `fr-FR-Denise`, `de-DE-Katja`, `ar-MA-Mouna`,
  `zh-CN-Xiaoxiao`, `en-US-Jenny`. It talks to the same service Edge's own Read Aloud uses
  (`src-tauri/src/tts.rs`), which is why Catalan and Moroccan Arabic work although Windows
  ships no local voice for either. That service is undocumented and Microsoft can change
  it; when it fails the app falls back to the local Web Speech voice and says so under the
  headword, and if there is no local voice for the language either, it says that instead.
  Clips are cached in memory for the session. If the button ever starts failing everywhere
  with a 403, the version constant in `tts.rs` has fallen behind current Edge — see the
  comment there.
- **rec** plays the Wiktionary contributor recording, fetched from Wikimedia on click.

### Dictionary packs

Packs are **generated artifacts and are not committed** — only the tiny authored
`packs/vertical-slice.sqlite` test fixture is in git. Build them locally; the app discovers
every `.sqlite` file in its pack directory at startup.

```sh
python scripts/build_all_packs.py            # all seven, smallest download first
python scripts/build_all_packs.py ca es      # just these
python scripts/fetch_and_build_pack.py en    # one target
```

| Target | Source | Download |
| --- | --- | --- |
| `en` | English Wiktionary | ~3.1 GB |
| `zh` | Chinese (Han headwords, plus Pinyin transcriptions) | ~1.1 GB |
| `de` | German | ~1.0 GB |
| `es` | Spanish | ~989 MB |
| `fr` | French | ~551 MB |
| `ca` | Catalan | ~230 MB |
| `ary` | Moroccan Arabic (Darija) | ~6 MB |
| `simple-en` | Simple English Wiktionary — small, for testing | ~39 MB |

Every build prints a coverage report and warns when a field it expected is empty, so a pack
that would ship with 0% of a feature fails visibly instead of silently.

Pack order and which packs are on are remembered in `pack-settings.json` in the app's data
directory. Both used to reset at every launch.

### Using packs on another machine

Building needs the 7 GB of Wiktionary dumps; a second machine should get the finished packs
instead. Package them once, move the `release/` folder however you like (GitHub Release
assets, a USB drive, a synced folder), and install on the other side:

```sh
# On the machine that built them: gzip each pack, write SHA256SUMS and a README.
python scripts/package_packs.py                 # every pack
python scripts/package_packs.py ca es           # just these

# On the other machine, from a Golddig checkout: verify, unpack, integrity-check, install
# into the per-user directory the app reads at startup.
python scripts/install_packs.py --dir path/to/release
python scripts/install_packs.py kaikki-catalan.sqlite.gz kaikki-spanish.sqlite.gz
```

`golddig --pack-dirs` prints every directory the app searches and which one to use —
`%APPDATA%\com.golddig.app\packs` on Windows. Dropping `.sqlite` files there by hand works
too; the install script only adds checksum verification and an integrity check.

Measured with `package_packs.py` over all seven packs: **5.25 GB unpacked, 1.24 GB to
download**, every file under GitHub's 2 GB release-asset limit.

| Pack | Entries | Unpacked | Download |
| --- | ---: | ---: | ---: |
| English | 1,491,592 | 1,674 MB | 500 MB |
| Chinese | 194,134 | 1,149 MB | 257 MB |
| Spanish | 810,914 | 905 MB | 180 MB |
| German | 371,241 | 1,001 MB | 169 MB |
| French | 403,164 | 444 MB | 89 MB |
| Catalan | 198,460 | 200 MB | 42 MB |
| Darija | 2,342 | 4 MB | 1 MB |

Most people want two or three of these, not all: English + Catalan + Spanish is 722 MB.

**Where translations come from.** English Wiktionary publishes translation tables only on
*English* lemmas (english to ca/es/fr/de/zh and so on). A Catalan or Spanish entry carries an
English gloss but no translations array — verified: 0 of the first 40,000 Catalan records have
one. So the `en` pack is what supplies the translations the UI shows. Translating *between*
two non-English languages needs a genuinely bilingual source (FreeDict TEI, Apertium); that
is tracked in [`docs/roadmap.md`](docs/roadmap.md) and is not implemented.

### Example sentences from Tatoeba

`golddig-pack tatoeba` merges [Tatoeba](https://tatoeba.org/downloads) sentences into an
existing pack, matching each sentence against that pack's headwords and inflected forms and
carrying the linked English translation:

```sh
golddig-pack tatoeba packs/kaikki-catalan.sqlite sentences.csv links.csv ca 3
```

### Quick Launchers & Shortcut

For quick access on Windows, open the [`launchers/`](launchers/) folder in File Explorer:

- **`launchers/create-shortcut.bat`**: Generates `Golddig.lnk` for *this* machine. The
  shortcut itself is not committed — a `.lnk` embeds an absolute path, so a committed one
  points at whichever machine generated it.
- **`launchers/start-golddig.bat`**: Double-click to launch the desktop app, building it
  first if the binary is missing.
- **`launchers/start-dev-mode.bat`**: Starts the live development server with hot-reloading.
- **`launchers/run-tests-and-benchmarks.bat`**: Runs the entire test suite and latency benchmark in a console window.

### Command Line

```sh
npm install
npm run check
npm run test
npm run build        # required before the Rust steps: tauri::generate_context! reads dist/
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Benchmark. Takes the packs to measure; with no argument it measures every pack in packs/.
cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-bench
cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-bench -- --fixture
```

The same steps run in CI on Linux and Windows — see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml).

### Ingestion CLI (`golddig-pack`)

Builds, enriches and inspects packs. Prefer `scripts/fetch_and_build_pack.py` for builds —
it rebuilds this binary first, generates the manifest from the real dump (URL,
`Last-Modified`, source SHA-256) and reports coverage. Use the binary directly for
inspection and for one-off builds.

```sh
PACK="cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-pack --"

# Build from authored JSONL entries
$PACK jsonl fixtures/vertical-slice.manifest.json \
           fixtures/vertical-slice.entries.jsonl packs/my-pack.sqlite

# Build from a raw Kaikki Wiktextract dump (manifest must match the dump's language)
$PACK kaikki fixtures/kaikki-ca.manifest.json fixtures/kaikki-ca.jsonl \
             packs/kaikki-catalan.sqlite kaikki-wiktionary [max_entries]

# Merge Tatoeba example sentences into an existing pack
$PACK tatoeba packs/kaikki-catalan.sqlite sentences.csv links.csv ca 3

# Entry count, manifest and search-term breakdown
$PACK inspect packs/kaikki-catalan.sqlite

# Run the real engine over the real packs and print the ranked result. Use this to check
# ranking against shipped artifacts, not just the synthetic packs in the test suite.
$PACK lookup man
$PACK lookup nihao 5
```

An unknown mode exits non-zero rather than falling through to a different command.

## Benchmarks

See [`docs/benchmarks/vertical-slice.md`](docs/benchmarks/vertical-slice.md). Figures are
reported per pack alongside that pack's size and entry count: numbers measured on the
7-entry authored fixture are not the engine's performance on a real dictionary.

Start the desktop application in development mode with `npm run tauri dev`.

## Important distinction

Wiktionary is the natural primary dictionary source. Wikipedia is an encyclopedia: it can later be an optional article source or a corpus for collocation statistics, but it should not be the foundation of lexical entries.

## Repository policy

Generated databases, downloaded dumps, and audio do not belong in Git. Data packs must be reproducible from pinned upstream snapshots and must carry attribution/license manifests.
