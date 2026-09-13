# Proposed architecture

Status: **proposal for the first implementation spike**

> ## What is actually built
>
> This document is a design proposal and large parts of it are not implemented. It was
> cited as if it described the shipped system, so here is the real inventory.
>
> **Built:** Tauri 2 shell; Rust core with `rusqlite`; Svelte 5 + TypeScript UI; one
> immutable SQLite file per pack; multi-pack discovery with per-pack enable/disable; a
> streaming Kaikki/Wiktextract importer; a Tatoeba example merger; language-aware
> normalization with loose keys; globally ranked multi-pack search with prefix, exact,
> wildcard, loose and content tiers; the `golddig-pack` and `golddig-bench` CLIs.
>
> **Pack schema as built** — four tables plus a full-text index, not the relational model
> described below:
>
> ```sql
> manifest(key, value)          -- 'manifest' (JSON) and 'default_source_id'
> sources(id, name, url, license, attribution)
> entries(id, language, lemma, pos, data_json)
>                               -- the entry is a JSON payload; records whose source_id
>                               -- equals default_source_id omit it, restored on read
> search_terms(entry_rowid, term, term_type, loose_key, term_rev)
>                               -- entry_rowid: INTEGER -> entries.rowid
>                               -- term_type: lemma | form | transcription
>                               -- term_rev: term reversed, for suffix probes
> entries_fts(headwords, definitions, examples, relations)
>                               -- contentless FTS5, rowid = entries.rowid
> ```
>
> Two size decisions are deliberate. `search_terms` references the entry by integer rowid,
> not its text id: the id is a string like `ca:noun:col·lecció` and was previously stored in
> the table and each of its indexes — five copies. And a record's `source_id` is omitted from
> the JSON when it equals the pack default, since storing the same id on every sense, example,
> form and pronunciation was 12.6% of the payload. Together with dropping an unused `language`
> column and a redundant `UNIQUE` index, the Catalan pack went from 273.8 MB to 199.7 MB
> (−27%) and its gzipped download from 48.9 MB to 41.8 MB, with no change in lookup latency.
> The engine detects either format at open time, so older packs still work.
>
> **Also built since this inventory was first written:**
>
> - **FTS5, as `entries_fts`.** Contentless, `unicode61 remove_diacritics 0` exactly as this
>   document always specified, over headwords / definitions / examples / related terms, ranked
>   with bm25 weighted in that order. Phrase lookup went from an unindexed `LIKE` scan of the
>   JSON payload (812 ms on a 198k-entry pack, 10.5 s on the 1.5M-entry English pack) to
>   **0.33 ms**, for about +5% pack size. Diacritic *tolerance* is still an explicit Rust rule
>   in `normalization.rs` — the tokenizer deliberately does not fold, so Spanish ñ and n stay
>   distinct. Packs built before the index exists still open and fall back to the old scan.
> - **Reader-settable pack priority**, persisted to `pack-settings.json` in the app data
>   directory, which breaks ranking ties between packs and also persists enable/disable.
>
> **Not built.** Do not read these as delivered:
>
> - **Relational senses.** There are no `sense`, `definition`, `translation`, `example`,
>   `collocation`, `relation`, `pronunciation` or `provenance` tables. Those live inside
>   `entries.data_json`.
> - **`catalog.sqlite`.** No writable catalog database, no global cross-pack term index, no
>   history, no bookmarks. Pack preferences live in a small JSON file instead; the rest of what
>   a catalog would hold does not exist yet, and designing its schema before it does would be
>   speculative.
> - **`.gdpkg` packages.** No transport archive, no signing, no side-by-side versions, no
>   atomic activation, no rollback, no sideload flow. A pack is a bare `.sqlite` file.
> - **Sandboxed import.** `golddig-pack` is a separate binary but runs unsandboxed with no
>   archive-expansion or resource limits.
> - **HTML sanitization.** Not needed yet: no importer emits markup. Required before any
>   StarDict/DSL/MDict adapter lands.
> - **StarDict, DSL, Dictd, MDict, XDXF, ZIM importers.** None exist.
> - **Frequency and statistical collocations.** The `collocations` field holds Wiktionary
>   related/derived/coordinate terms, which are editorial relations, not corpus statistics.
>
> Provenance is partial: each pack manifest records the source URL, the dump's
> `Last-Modified` date and a SHA-256 of the downloaded file, and every sense carries a
> `source_id`. There is no per-fact provenance row and no output hash.

## Recommendation

Use **Tauri 2 + Rust + Svelte/TypeScript + SQLite FTS5**.

Tauri uses the operating system webview rather than shipping a private Chromium runtime. Rust is a good fit for streaming multi-gigabyte source files, deterministic data-pack builds, compact native binaries, and concurrent lookup. Svelte keeps the UI layer small. SQLite provides a portable, inspectable file format and fast local indexed reads.

Bundle a known SQLite build with FTS5 and access it only through the Rust core using `rusqlite`; do not expose a generic SQL plugin to the webview. Run blocking SQLite work on a bounded worker pool and expose narrow typed commands such as `search`, `load_entry`, and `install_pack`.

Rust is not currently installed on the development machine, so toolchain installation is a prerequisite once this proposal is accepted. Node.js and npm are already available.

## Component model

```text
Upstream snapshots
  Kaikki JSONL / FreeDict TEI / Tatoeba TSV / WordNet LMF
                 |
          source adapters
                 |
    validation + normalization + provenance
                 |
       immutable SQLite data packs
                 |
      Rust query/aggregation service
                 |
       Tauri commands/events boundary
                 |
        Svelte result workspace
```

### Application core

Responsibilities:

- pack catalog and compatibility checks;
- language-aware query normalization;
- concurrent lookup across installed packs;
- ranking and deduplication;
- attribution/provenance retrieval;
- audio cache management;
- update/download verification.

The core should not contain dictionary content.

### Content security boundary

Treat imported markup, links, and media metadata as untrusted data. Convert source content to a small text-first rendering model—headings, paragraphs, labels, lists, definitions, examples, emphasis, IPA, and links. One Golddig stylesheet controls presentation. Do not execute or inherit source CSS/JavaScript, and do not render images in the initial result surface. Pronunciation audio is an optional user-invoked inline control and must not block or be required for text lookup.

When a legacy dictionary contains only opaque HTML, sanitize it in the importer and map its supported structure into the safe intermediate model. Discard scripts, event handlers, remote fonts, tracking resources, unsafe CSS, and unsupported decorative media. Dictionary articles never gain Tauri command access.

### Source adapters

Separate structured source adapters from user-dictionary format importers.

Structured source adapters:

1. Kaikki/Wiktextract JSONL;
2. FreeDict TEI XML;
3. Tatoeba TSV exports;
4. WordNet LMF.

User-dictionary formats:

1. StarDict as the first complete legacy adapter;
2. ABBYY DSL early, with an explicitly documented supported-tag subset;
3. Dictd as a comparatively simple proving ground;
4. unencrypted MDict after the native model and safe renderer are proven;
5. XDXF, ZIM, and other formats according to demand.

Interoperability should be independently implemented from public format documentation unless Golddig deliberately adopts a GPL-compatible codebase license. Large or untrusted imports should run in a separate `golddig-pack` process with archive expansion limits, path-traversal protection, maximum field/resource sizes, MIME checks, HTML sanitization, cancellation, and SQLite integrity checks before installation.

## Data-pack model

Use one immutable SQLite file per source/snapshot/language group. The app keeps a small writable `catalog.sqlite` for settings, installed-pack metadata, history, bookmarks, and a compact global lookup index that maps normalized terms to pack IDs and record IDs.

Do **not** attach every pack to one SQLite connection: SQLite's default attached-database limit is 10, and users may install many more packs. Query the global catalog first, then read candidate records through bounded read-only connection pools for the relevant pack files. Open immutable pack files read-only and verify their hashes before cataloging them.

Benefits:

- users install only what they need;
- updates are signed, hash-verified atomic file swaps;
- source licenses and attribution remain separable;
- broken packs are easy to remove;
- pack builders can be reproduced independently;
- launch uses the persistent catalog instead of rescanning or rebuilding every dictionary index.

Distribute packs as `.gdpkg` transport archives containing `manifest.json`, `dictionary.sqlite`, optional content-addressed `assets/`, and a signature. Keep the installed SQLite file uncompressed for random access, and install versions side by side so Windows updates can transactionally switch the active version and roll back without replacing an open file.

Every pack must include a manifest containing at least:

- pack ID, semantic/content version, schema version, and normalization version;
- source name and canonical URL;
- source snapshot/revision and retrieval date;
- input, database, resource, archive, and output cryptographic hashes;
- languages and features present;
- license identifier, license URL/text, and attribution;
- importer name/version and transformation recipe;
- minimum and maximum compatible Golddig/schema versions;
- signing key ID and signature.

## Common lexical model

Do not force all sources into a single flattened article. Preserve both normalized fields and source-native payload/provenance.

Core entities:

- `entry`: language, lemma, normalized keys, part of speech, source entry ID;
- `form`: written form, grammatical tags, normalized keys;
- `sense`: ordered glosses, labels/register/region, source sense ID;
- `translation`: source sense, target language, term, tags;
- `example`: text, translation, source/license/author IDs;
- `relation`: synonym, antonym, hypernym, hyponym, derived/related term;
- `pronunciation`: IPA, region tags, audio reference and media license;
- `collocation`: phrase/partner, relation type, score, corpus/source;
- `frequency`: score, scale, corpus/source, region and date;
- `provenance`: source, snapshot, license, attribution, original record ID.

Keep a JSON source fragment only where needed for lossless debugging; do not make lookup depend on decoding large JSON blobs.

## Search strategy

### Indexed keys

Store multiple keys rather than destroying spelling distinctions:

1. exact NFC form;
2. Unicode case-folded form;
3. diacritic-tolerant fallback;
4. language-specific alternate key;
5. prefix-search key;
6. all inflected forms linked to their lemma.

Configure FTS5 with `unicode61 remove_diacritics 0` and precomputed loose keys. This prevents SQLite from silently conflating distinctions such as Spanish `ñ`/`n`; diacritic-tolerant behavior remains an explicit, testable, lower-ranked application rule. Use prefix indexes for suggestions and contentless per-pack FTS tables to avoid duplicating all rendered text. Escape or construct FTS queries rather than passing raw user input directly to `MATCH`.

Language-specific fallbacks should include tests for:

- English apostrophes, hyphens, and American/British spelling labels;
- Catalan apostrophes, accents, and `l·l`/`ll`, without flattening useful distinctions at the exact tier;
- French elision/apostrophes, ligatures such as `œ`/`oe`, and accents;
- German `ß`/`ss` and optional `ä/ae`, `ö/oe`, `ü/ue` fallbacks;
- Spanish accented vowels and `ü`, while keeping `ñ` distinct from `n` in the normal loose tier.

Collision tests are mandatory: `ma`/`mà`, `ano`/`año`, `ll`/`l·l`, `Straße`/`Strasse`, and `cœur`/`coeur`. A broader transliteration tier may be optional, but it must never make a lossy fallback rank like an exact match.

### Ranking

Suggested order:

1. exact lemma;
2. exact inflected form;
3. case-folded lemma;
4. language-specific alternate;
5. prefix completion;
6. fuzzy candidates reranked in Rust.

Within a result, rank by requested language, region (prefer American English when relevant), lexical frequency, entry quality, and source priority. Never hide the source.

## Result UX

The result surface is a restrained single reading column whose hierarchy comes from typography and whitespace, not image-heavy cards. It should render progressively as providers answer:

- header: lemma, language, POS, IPA, and an optional compact US-audio control;
- compact translation line near the top;
- numbered definitions grouped by source and POS;
- indented examples directly beneath the relevant source sense when alignment exists;
- synonyms/relations and common combinations as compact text lists;
- forms and usage labels;
- clear `Exact`, `Normalized`, `Morphology`, and `Fuzzy` text labels;
- subtle source separators and expandable attribution details;
- optional compact lookup through a global shortcut; clipboard monitoring remains off by default, visibly indicated when enabled, and never creates clipboard history.

Avoid source-supplied layout, thumbnails, banners, decorative images, deep card chrome, gratuitous badges, and layout shifts. Render the first useful source immediately, add remaining source sections without moving the reading position, collapse pathological entries after a useful preview, and do not mount hidden content.

“Everything at once” should mean a coherent workspace, not one giant untraceable merged article.

## Performance targets for the first spike

- bounded-memory streaming import;
- cold application start under 1 second on the development Windows machine;
- exact local lookup under 30 ms at the query layer;
- first useful formatted text under 100 ms after window readiness;
- prefix suggestions under 50 ms;
- responsive selection, copying, collapse, and navigation on pathological multi-source entries;
- no image, audio download, update check, full-text indexing, or network request on the critical path to visible text;
- no network request during ordinary lookup;
- reproducible pack output from a pinned input fixture.

These are targets to measure, not claims about an implementation that does not exist yet.
