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
> **The four next pieces, in priority order:**
>
> 1. **FTS5 for content search.** Phrase lookup currently falls back to `LIKE` over the JSON
>    payload — ~680 ms on a 198k-entry pack. A contentless FTS5 index over definitions and
>    examples would make it a fast ranked tier instead of a slow last resort, and is the one
>    piece of `docs/architecture.md` worth actually building.
> 2. **Real bilingual translations.** English Wiktionary only publishes translation tables on
>    *English* lemmas, so the `en` pack supplies english→X and nothing supplies X→Y. The fix
>    is a genuinely bilingual source:
>    - **FreeDict** (<https://freedict.org/>) — TEI XML, many pairs including eng↔spa,
>      eng↔fra, eng↔deu, eng↔cat. Licences vary **per dictionary**; check each.
>    - **Apertium** bilingual dictionaries (<https://github.com/apertium>) — GPL, strong
>      ca↔es coverage, built for exactly this.
>    - **CC-CEDICT** (<https://cc-cedict.org/>) — CC BY-SA 4.0, ~120k entries, the standard
>      English↔Mandarin source with Pinyin. Better for zh than Wiktionary alone.
> 3. **Statistical collocations.** What the UI labels "Commonly used with" is currently
>    Wiktionary related/derived/coordinate terms — editorial relations, *not* corpus
>    statistics. Real collocations need co-occurrence counts from a corpus: the
>    **Leipzig Corpora Collection** (<https://wortschatz.uni-leipzig.de/en/download>)
>    publishes per-language co-occurrence tables, licence per corpus. Until then the label
>    overstates what the data is and should be read as "related words".
> 4. **Offline neural pronunciation.** The speaker buttons are the platform Web Speech API,
>    which has no Catalan or Moroccan Arabic voice on Windows and whose best voices are
>    cloud-only. Two honest routes: ship the Wiktionary recordings the importer already
>    preserves in `PronunciationRecord.audio`, or bundle **Piper** ONNX voices
>    (<https://github.com/rhasspy/piper>, MIT, ~20–60 MB per voice) invoked from Rust for
>    genuinely offline synthesis.
>
> **User-settable pack priority.** With eight packs enabled, a short query matches many
> entries equally well: `man` is an exact lemma in English (noun, verb, pronoun,
> interjection), German (pronoun, adverb), French, Spanish and Chinese all at once. Ranking
> now prefers the headword spelled as typed and demotes proper nouns and affixes, which fixes
> the clearly-wrong cases. Beyond that the order falls to pack load order, which is
> alphabetical and arbitrary. Which language should win an exact tie is a reader preference,
> not something the engine can infer, so it needs an explicit per-pack priority the reader can
> reorder — GoldenDict solved this with dictionary groups. `PackInfo` would carry a `priority`
> and the packs panel would allow drag-ordering.
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
