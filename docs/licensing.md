# Licensing strategy

This is an engineering policy, not legal advice.

## Separate three things

1. **Golddig application code** — choose one explicit open-source software license.
2. **Importer/build recipes** — normally use the application license or another explicit software license.
3. **Dictionary/audio packs** — retain the upstream content licenses and attribution obligations; never imply that the application license relicenses the data.

## Application license

Golddig application code is licensed under **MPL-2.0**. MPL provides file-level copyleft: modified MPL-covered files remain open while independently licensed modules and separately distributed data packs can retain compatible independent terms. See the repository root `LICENSE`.

Dictionary/audio content is not relicensed under MPL-2.0. Each pack retains its upstream content license and attribution obligations.

## Pack rules

- A pack may contain only licenses that can coexist under its distribution model.
- Share-alike transformed data stays in independently distributable pack files.
- Attribution must be available in the UI and included in the pack manifest/distribution.
- Store record-level source/author/license IDs where obligations differ by record, especially audio and sentence corpora.
- Publish source URLs, pinned revisions/dates, hashes, and build recipes.
- Offer corresponding transformed source/data where a license requires it.
- Treat “free download” as insufficient; verify permission to modify and redistribute.

## GoldenDict compatibility

GoldenDict and GoldenDict-ng are GPLv3 projects. Reading documented dictionary formats with an independently written importer does not require copying their implementation. If Golddig copies or adapts their GPL-covered code, Golddig's distributed combined work must comply with GPLv3-compatible terms.

## Initial source notes

- Kaikki states that its Wiktionary-derived data is available under the same licenses as Wiktionary (CC BY-SA and GFDL). Preserve snapshot and attribution information.
- Tatoeba's general sentence exports are CC BY 2.0 FR; a separate CC0 subset exists. Audio licenses are contributor-selected and must be checked per audio record.
- FreeDict dictionaries are free/open, but the exact license is dictionary-specific and must be read from source metadata.
- `wordfreq` code is Apache-2.0; its included data files are offered under CC BY-SA 4.0 with additional source acknowledgements documented by the project.
- Open English WordNet is derived from Princeton WordNet under the WordNet license and further developed under CC BY 4.0.
- OdeNet data is CC BY-SA 4.0; its Python code is MIT.
- Wikimedia Commons media is freely licensed, but the exact license and attribution are file-specific.
