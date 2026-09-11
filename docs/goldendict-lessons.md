# Lessons from GoldenDict and GoldenDict-ng

Golddig should treat GoldenDict as a **product reference**, not automatically as a codebase to port.

## Preserve

- instant headword search and suggestions;
- multiple dictionaries visible in one text-first result workspace;
- exact, morphology-assisted, accent/case-tolerant, and fuzzy lookup;
- dictionary groups/source enablement and ordering;
- readable article hierarchy—headings, labels, definitions, examples, cross-references, and optional inline audio—without image-heavy or source-controlled layouts;
- global shortcuts, clipboard/selection popup, history, favorites, and headword browsing;
- local dictionaries first, with optional web/program sources;
- broad interoperability with user-owned dictionary files.

## Improve

- show provenance and license at the fact/card level rather than hiding source boundaries;
- install data as versioned packs instead of rescanning every dictionary root at launch;
- build indexes during pack creation/install, not automatically at every startup;
- render imported content through a safe structured model, not unrestricted article JavaScript;
- make audio optional and separately cached;
- provide deterministic updates with signatures, hashes, and rollback;
- use a coherent result layout rather than simply concatenating arbitrary HTML pages;
- make regional preferences, especially American English pronunciation/spelling, explicit ranking inputs.

## Avoid in the first release

- dozens of legacy formats before the native data model works;
- automatic full-text indexing of huge encyclopedias or user dictionaries at startup; make per-dictionary FTS opt-in with disk estimates, progress, pause, cancel, and delete controls;
- arbitrary remote websites inside a privileged app webview;
- hidden network requests during local lookup;
- always-on clipboard monitoring or clipboard history;
- leading-wildcard scans without a suitable index and an explicit warning;
- aggressive cross-source sense merging;
- bundled dictionaries whose redistribution rights are unknown;
- assuming MDict/Babylon/DSL file availability implies content may be redistributed.

## Adapter order

Keep two tracks distinct.

### Structured data sources

1. **Kaikki JSONL** — validates Golddig's native structured model.
2. **FreeDict TEI** — open bilingual data and a strong canonical XML source.
3. **Tatoeba TSV** and **WordNet LMF** — examples, translations, and semantic relations.

### User dictionary formats

1. **StarDict** — first complete legacy adapter; support `.ifo`, `.idx`, `.dict`/`.dict.dz`, `.syn`, and companion `res/`, `res.zip`, or basename `.res.zip` resources.
2. **ABBYY DSL** — ship early; support `.dsl`/`.dsl.dz`, abbreviations, companion resources, and a documented subset of tags with clear unsupported-tag reporting.
3. **Dictd** — comparatively simple and useful as an inexpensive proving ground.
4. **MDict** — highest-priority post-MVP rich format; begin with unencrypted, lawfully obtained files, safe HTML/media handling, and 64-bit offsets for files over 4 GB.
5. **XDXF** — a useful open XML format after the internal article model is stable.
6. **ZIM and Slob** — later optional encyclopedia/reference support.
7. **Babylon BGL and other niche/proprietary formats** — defer because of parser, encoding, resource, and licensing complexity.

If Golddig launches with only three prominent user-facing families, the strongest candidates are **StarDict, DSL, and MDict**, while Dictd can be implemented earlier as an internal proving ground.

## GPL boundary

GoldenDict and GoldenDict-ng are GPLv3-or-later projects. If Golddig copies, ports, translates, or closely adapts their covered parser, UI, or indexing code, distributed combined work must follow GPL-compatible terms and corresponding-source obligations. Dynamic linking is not a reliable escape from that requirement.

Clean-room interoperability is the recommended default: use public format specifications, independently authored fixtures, black-box compatibility tests where lawful, and no copied implementation. If the project chooses GPL-3.0-or-later deliberately, reuse can be reconsidered with preserved notices and a dependency/license audit.

This is an engineering summary, not legal advice.
