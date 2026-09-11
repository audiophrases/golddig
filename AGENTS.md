# Golddig development rules

## Verify the artifact, not the source

Read this first. Every one of the following shipped to a user at once, and all of them were
reported as delivered, because work was checked by reading the code and running unit tests
against a 10-entry authored fixture — never by querying a built pack.

- Translations and collocations were 0% in every real pack while the parser that extracts
  them was correct and unit-tested. The packs had been compiled by an older binary and never
  rebuilt, and the fixture the tests used had 100% coverage of both.
- The "Mandarin" pack contained 6 Chinese characters out of 72,909 entries. The pipeline
  pointed at the pinyin romanization index instead of the Chinese extract.
- The `en` target pointed at Simple English Wiktionary (39 MB) while the manifest said
  "English Wiktionary" (3.1 GB).
- The Pinyin/romanization search index had zero rows in every pack: the importer gated it on
  a JSON field name, `zh_pron`, that does not exist in the data (it is `zh-pron`).
- `golddig-pack tatoeba` printed a success message and created nothing.
- Every prefix query full-scanned the term table on every keystroke, while the published
  benchmark — measured on the 10-entry fixture — reported 0.12 ms.
- All nine CSS custom properties the UI paints with were undefined, so the app was unstyled.
- The installers shipped no dictionaries and the status bar said "packs loaded" regardless.

The rules that follow from that:

1. **A feature is not done until you have queried a shipped artifact and seen it.** Not the
   fixture, not the source, not a test — the `.sqlite` file a user would get.
2. **Rebuild the producer before you trust its output.** `scripts/fetch_and_build_pack.py`
   always recompiles `golddig-pack` for exactly this reason. Do not bypass it.
3. **Measure coverage and report the number.** Every pack build prints per-field coverage and
   warns on an empty field. If a number is 0%, say so and find out why before moving on.
4. **Benchmark the real thing, and name what you measured.** Fixture numbers are a floor, not
   a result. State the pack, its size, and its entry count next to every figure.
5. **If a test passes while the product is broken, the test is the bug.** Add one that fails.
   `test_shipped_packs_are_structurally_sound` exists because nothing opened a built pack.
6. **Do not report a count, a latency, or a coverage figure you did not just observe.** Quote
   the command output.
7. **Run the app.** Several of these were visible in the first second of looking at the window.

## Product constraints

- Keep ordinary lookup local-first, fast, and usable without an account or network connection.
- Initial languages are English (`en`), Catalan (`ca`), French (`fr`), German (`de`), and Spanish (`es`).
- Preserve source boundaries in the UI. Do not automatically merge senses from different providers unless alignment is backed by an explicit identifier or a tested alignment rule.
- Keep application binaries separate from optional dictionary/audio data packs.
- Keep the result surface text-first: semantic formatted text, restrained typography, and whitespace. Do not add image-heavy cards, decorative media, source-supplied CSS/JavaScript, or animation-heavy UI.
- Pronunciation audio may remain an optional inline control; it must not make audio packs or network access necessary for text lookup.
- Treat Golddig as independent from GoldenDict/GoldenDict-ng; do not reuse their branding or imply affiliation.
- Clipboard monitoring is opt-in, visibly indicated, and must not create a clipboard history.

## Data and licensing

- Never commit downloaded dictionary dumps, generated databases, or bulk audio (except small authored test fixtures like `packs/vertical-slice.sqlite`).
- Every importer must record source URL, source version/date, retrieval date, content hash, license identifier/text URL, attribution, transformation recipe version, and output hash.
- Never assume a project-wide license covers every record or media file. Audio and FreeDict dictionaries can have per-item/per-dictionary terms.
- Do not redistribute a data pack until its license and attribution path are verified.
- Keep transformed share-alike data separable from application code and from data under incompatible terms.
- Prefer reproducible import recipes and pinned snapshots over opaque prebuilt databases.

## Engineering

- Benchmark cold start, exact lookup, prefix lookup, and database size before broadening scope.
- Exact spelling/accent matches rank above normalized fallbacks.
- Fuzzy search is a fallback, never the first query path.
- Importers stream records and use bounded memory.
- Add fixture-based tests for every importer; fixtures must be authored or legally redistributable and include provenance.
- Update README and relevant docs after non-trivial architecture, data, or workflow changes.
