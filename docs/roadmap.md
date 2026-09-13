# Initial roadmap

> ## Current state, and what is actually next
>
> Phases 0 and 1 are done. Phase 2 is partly done. The phase descriptions below are the
> original plan and still read as aspirational in places — this block is the honest status.
>
> **Working:** Tauri 2 desktop app; Rust core; Svelte 5 text-first UI with keyboard
> navigation; seven buildable language packs (en, ca, es, fr, de, ary, zh) plus a committed
> test fixture; streaming Kaikki importer carrying definitions, IPA with region tags, audio
> URLs, inflections, synonyms, related/derived terms, romanized transcriptions and register
> labels; a Tatoeba example merger; globally ranked multi-pack search with exact, prefix,
> wildcard (`?`/`*`, including suffix patterns via a reversed-term index), loose
> diacritic-folded and content tiers; per-pack enable/disable; CI on Linux and Windows.
>
> **Done since:** a contentless FTS5 index (`entries_fts`) over headwords, definitions,
> examples and related terms, making phrase lookup a fast ranked tier — **812 ms → 0.33 ms**
> on a 198k-entry pack, for +4.9% pack size. And reader-settable pack priority, persisted to
> `pack-settings.json` in the app data directory, which also finally persists pack
> enable/disable across restarts.
>
> **The three next pieces, in priority order:**
>
> 1. **Real bilingual translations.** English Wiktionary only publishes translation tables on
>    *English* lemmas, so the `en` pack supplies english→X and nothing supplies X→Y. The fix
>    is a genuinely bilingual source:
>    - **FreeDict** (<https://freedict.org/>) — TEI XML, many pairs including eng↔spa,
>      eng↔fra, eng↔deu, eng↔cat. Licences vary **per dictionary**; check each.
>    - **Apertium** bilingual dictionaries (<https://github.com/apertium>) — GPL, strong
>      ca↔es coverage, built for exactly this.
>    - **CC-CEDICT** (<https://cc-cedict.org/>) — CC BY-SA 4.0, ~120k entries, the standard
>      English↔Mandarin source with Pinyin. Better for zh than Wiktionary alone.
> 2. **Statistical collocations.** What the UI labels "Commonly used with" is currently
>    Wiktionary related/derived/coordinate terms — editorial relations, *not* corpus
>    statistics. Real collocations need co-occurrence counts from a corpus: the
>    **Leipzig Corpora Collection** (<https://wortschatz.uni-leipzig.de/en/download>)
>    publishes per-language co-occurrence tables, licence per corpus. Until then the label
>    overstates what the data is and should be read as "related words".
> 3. **Offline neural pronunciation.** The speaker buttons now use Microsoft's neural voices
>    through the Edge Read Aloud service (`src-tauri/src/tts.rs`), which covers every pack
>    language including Catalan and Moroccan Arabic — but it is a reverse-engineered
>    endpoint that needs the network and could be withdrawn. The Wiktionary recordings the
>    importer preserves in `PronunciationRecord.audio` play from the `rec` button (also a
>    network fetch, from Wikimedia). Genuinely offline synthesis would mean bundling
>    **Piper** ONNX voices (<https://github.com/rhasspy/piper>, MIT, ~20–60 MB per voice)
>    invoked from Rust; nothing is built for that yet.
>
> **Pack priority — built, with one rough edge.** With eight packs enabled, a short query
> matches many entries equally well: `man` is an exact lemma in English (noun, verb, pronoun,
> interjection), German (pronoun, adverb), French, Spanish and Chinese all at once. Ranking
> prefers the headword spelled as typed and demotes proper nouns and affixes; beyond that the
> reader's pack order decides, set with the arrows in the packs panel and persisted. The rough
> edge is the control itself: arrows rather than drag-and-drop, and no grouping, so there is no
> way to say "Catalan and Spanish together, English after". GoldenDict's dictionary groups are
> the richer model if this proves too blunt.
>
> **Also outstanding:** Open English WordNet and OMW for synsets; `wordfreq` for commonness;
> `.gdpkg` packaging with signing and atomic activation; StarDict/DSL/Dictd/MDict importers;
> history and bookmarks; a writable catalog database.

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
