# Golddig development rules

## Product constraints

- Keep ordinary lookup local-first, fast, and usable without an account or network connection.
- Initial languages are American English (`en-US`), Catalan (`ca`), French (`fr`), German (`de`), and Spanish (`es`).
- Preserve source boundaries in the UI. Do not automatically merge senses from different providers unless alignment is backed by an explicit identifier or a tested alignment rule.
- Keep application binaries separate from optional dictionary/audio data packs.
- Keep the result surface text-first: semantic formatted text, restrained typography, and whitespace. Do not add image-heavy cards, decorative media, source-supplied CSS/JavaScript, or animation-heavy UI.
- Pronunciation audio may remain an optional inline control; it must not make audio packs or network access necessary for text lookup.
- Treat Golddig as independent from GoldenDict/GoldenDict-ng; do not reuse their branding or imply affiliation.
- Clipboard monitoring is opt-in, visibly indicated, and must not create a clipboard history.

## Data and licensing

- Never commit downloaded dictionary dumps, generated databases, or bulk audio.
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
