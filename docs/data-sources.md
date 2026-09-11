# Open lexical-data source survey

Researched: 2026-09-11. Verify the exact license and current snapshot again when building a distributable pack.

## Recommended source stack

| Source | Best use | Coverage | Format/update | License notes | Priority |
|---|---|---|---|---|---|
| [Kaikki / Wiktextract](https://kaikki.org/dictionary/rawdata.html) | Definitions, POS, senses, forms, IPA, audio URLs, translations, examples, synonyms/relations, usage/region tags | All five target languages through English Wiktionary; separate French, German, and Spanish edition extracts also exist | JSONL; raw English edition is updated regularly, usually at least weekly | Kaikki says extracted data follows Wiktionary licensing: CC BY-SA and GFDL. Wiktextract code is MIT. | **Primary** |
| [FreeDict](https://freedict.org/downloads/) | Bilingual translations and legacy interoperability | Many directions among English, Catalan, French, German, and Spanish | TEI source plus StarDict, Dictd, and Slob builds; static JSON API | Dictionary-specific free licenses; inspect each TEI/source package before redistribution | **High** |
| [Tatoeba downloads](https://tatoeba.org/en/downloads) | Natural example sentences and linked translations | All five languages, uneven quality/coverage | Weekly TSV/tar exports; custom language-pair exports | General sentence exports: CC BY 2.0 FR. Separate CC0 subset. Audio license is per contributor/file; blank means it cannot be reused outside Tatoeba. | **High** |
| [Open English WordNet](https://en-word.net/) | English synonyms, definitions, semantic relations | English | WN-LMF and related releases | Princeton WordNet license for underlying data; later work CC BY 4.0 | **High for English** |
| [CMU Pronouncing Dictionary](http://www.speech.cs.cmu.edu/cgi-bin/cmudict) | North American English phoneme pronunciations | 134k+ English words | Plain machine-readable lexicon | Open-source CMU distribution; record exact release/license text in the pack | **High for AmE** |
| [SCOWL / English Speller Database](https://wordlist.aspell.net/) | American spelling variants, commonness bands, basic POS/inflection | English dialect variants | Generated word lists/database | Multi-source licensing; use the project's generated license manifest for the selected list | Evaluate for AmE |
| [Softcatalà Catalan dictionary](https://huggingface.co/datasets/softcatala/catalan-dictionary) | Catalan spelling, forms, and POS/morphology support—not definitions | Catalan | Hunspell/LanguageTool-oriented source data | Dual GPL-2.0/LGPL-2.1 according to the project dataset/repository | **Useful for Catalan forms** |
| [OdeNet](https://github.com/hdaSprachtechnologie/odenet) | German synonyms and semantic relations | German | WordNet data | Data CC BY-SA 4.0; code MIT | **High for German** |
| [OpenThesaurus](https://www.openthesaurus.de/about/download) | German synonyms and associations | German | Text, MySQL, and LibreOffice/OpenOffice downloads | Data can be used under CC BY-SA 4.0 or GNU LGPL (user chooses), with source attribution | **High for German** |
| [Lexique 4](http://www.lexique.org/) | French frequency, contextual diversity, phonology, lemmas, and morphology | About 190k French word forms | Downloadable lexical database | CC BY-SA 4.0 | **High for French enrichment** |
| [WOLF](https://almanach.inria.fr/software_and_resources/WOLF-fr.html) | French synsets/semantic relations | French | French WordNet resource | CeCILL-C; verify redistribution/attribution details in the release | Evaluate |
| Multilingual Central Repository / compatible OMW resources | Catalan and Spanish synsets linked to English concepts | Catalan, Spanish, English | WordNet-family resources | Licenses differ by component/version; verify the exact downloadable release before inclusion | Evaluate carefully |
| [`wordfreq`](https://github.com/rspeer/wordfreq) | Commonness/frequency ranking | 40+ languages; all five target languages are listed, with Catalan in the project's highest source-count tier | Packaged frequency lists; project says underlying data is unlikely to receive further major updates | Code Apache-2.0; data CC BY-SA 4.0 with documented source acknowledgements | **Useful supplement** |
| [Leipzig Corpora Collection](https://corpora.uni-leipzig.de/) | Word frequencies and statistical co-occurrences/collocations | All five target languages have corpora; Catalan co-occurrence data is explicitly listed | Downloadable word/sentence/co-occurrence files | License can vary by corpus/download; accept only individually verified open datasets | Phase 3 |
| [Wikidata Lexemes](https://www.wikidata.org/wiki/Wikidata:Lexicographical_data) | CC0 forms, senses, and cross-language identifiers | All five, uneven completeness | RDF/JSON/SPARQL/dumps | Wikidata structured data is CC0; provenance to imported statements still matters | Optional |
| [Wikimedia Commons](https://commons.wikimedia.org/) / Lingua Libre | Pronunciation recordings | All five | Individual media files and metadata | License/author attribution is file-specific; do not treat the entire corpus as one blanket license | Audio supplement |
| Wikipedia dumps | Optional encyclopedia cards; corpus input for frequency/collocation experiments | All five | Large Wikimedia dumps | CC BY-SA; not a dictionary and expensive to process | Not primary |

## Why Kaikki first

Wiktextract already converts complex Wiktionary templates into line-oriented JSON. A sampled Catalan record was verified to contain a headword, language code, POS, sense gloss, synonyms, IPA, region tags, and Wikimedia audio URLs in one object. That is enough to validate most of Golddig's end-to-end model before integrating more sources.

The language-specific Kaikki pages under the **English Wiktionary edition** describe Catalan, French, German, Spanish, and English words primarily with **English glosses**. They are excellent for a shared first pipeline but do not by themselves provide native-language definitions. Kaikki also publishes raw extracts for the French, German, and Spanish Wiktionary editions; those editions can add French-, German-, and Spanish-language glosses. Catalan-native definitions need a separately validated Viccionari extraction path or another open Catalan lexical source; Softcatalà's open data is especially useful for spellings and morphology but is not a definition dictionary.

The current convenience per-language postprocessed downloads report approximately:

| Language | Size |
|---|---:|
| English | 3.0 GB |
| Catalan | 230.4 MB |
| French | 550.8 MB |
| German | 1.0 GB |
| Spanish | 989.4 MB |

Those per-language files are marked **deprecated** by Kaikki. They are acceptable for a short-lived feasibility spike, but a production pack builder should stream the current raw extract or run Wiktextract against a pinned Wikimedia dump.

## FreeDict coverage among the initial languages

The FreeDict static API currently lists every relevant direction, with very uneven sizes. Examples include:

- `eng-deu` about 460k headwords and `deu-eng` about 518k;
- `eng-spa` about 64k;
- `eng-cat` about 35k and `cat-eng` about 24k;
- 2025-generated Catalan/French/German/Spanish pairs mostly around 15k–60k;
- older English/French and Spanish/English pairs are much smaller.

Use the TEI source as the canonical import path. StarDict import should still be supported for user-owned dictionaries and interoperability.

## What each source does not solve

- **Kaikki/Wiktionary:** broad but community-edited, inconsistent across languages, and not a reliable statistical collocation source.
- **FreeDict:** excellent open bilingual plumbing, but quality, age, and license vary per dictionary.
- **Tatoeba:** examples are not sense-aligned by default; avoid attaching a plausible sentence to the wrong meaning.
- **WordNet family:** strong semantic relations, weaker for inflections, modern phrases, pronunciation, and direct bilingual display; multilingual sense links need careful validation.
- **Frequency data:** “common” depends on region, medium, date, and corpus. Store those dimensions instead of presenting a false universal rank.
- **Audio:** files are large and licenses vary; make audio a separate cached or downloadable pack.

## Proposed data tiers

### Core lexical pack

Kaikki/Wiktionary, split by lookup language, with definitions, translations, forms, examples, relations, IPA, and audio references.

### Enrichment packs

- FreeDict translations;
- Tatoeba examples/translations;
- language-specific WordNets;
- frequency data;
- statistically generated collocations;
- optional audio binaries.

### User import packs

StarDict first; later Dictd, DSL, ZIM, and MDict where specifications and legal constraints permit. Golddig can import user-provided content without redistributing it.

## Source-quality rules

- Display source and license per result card.
- Keep regional tags; prefer `US`/`General American` pronunciation and examples when the query language is American English.
- Preserve exact spelling and diacritics; normalization is only a search aid.
- Do not silently merge senses from different sources.
- Record upstream IDs so corrections can be traced back.
- Pin snapshots and build reproducibly; never scrape live pages into a release.

## Official links

- Kaikki raw downloads: <https://kaikki.org/dictionary/rawdata.html>
- Wiktextract repository/schema documentation: <https://github.com/tatuylonen/wiktextract>
- Wikimedia dumps: <https://dumps.wikimedia.org/>
- FreeDict downloads/API: <https://freedict.org/downloads/> and <https://freedict.org/freedict-database.json>
- Tatoeba downloads: <https://tatoeba.org/en/downloads>
- Open Multilingual Wordnet: <https://omwn.org/>
- Open English WordNet: <https://en-word.net/>
- CMU Pronouncing Dictionary: <http://www.speech.cs.cmu.edu/cgi-bin/cmudict>
- SCOWL / English Speller Database: <https://wordlist.aspell.net/>
- Softcatalà Catalan dictionary data: <https://huggingface.co/datasets/softcatala/catalan-dictionary>
- OdeNet: <https://github.com/hdaSprachtechnologie/odenet>
- OpenThesaurus downloads: <https://www.openthesaurus.de/about/download>
- Lexique 4: <http://www.lexique.org/>
- `wordfreq`: <https://github.com/rspeer/wordfreq>
- Leipzig corpora: <https://corpora.uni-leipzig.de/>
- GoldenDict-ng format overview: <https://xiaoyifang.github.io/goldendict-ng/dictformats/>
