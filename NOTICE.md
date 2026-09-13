# Notices and attribution

Golddig ships two legally distinct kinds of material, and this file covers both. It exists
because the project redistributes CC BY-SA dictionary data, and share-alike attribution has
to be available at the point of distribution — not only inside a database table.

## Golddig application code

Copyright the Golddig contributors. Licensed under the Mozilla Public License 2.0; see
[LICENSE](LICENSE). This covers `src/`, `src-tauri/src/`, `scripts/`, `tests/` and the
build configuration.

Golddig is an independent project. It is **not affiliated with, endorsed by, or derived
from GoldenDict or GoldenDict-ng.** Those projects are GPL-3.0-or-later; no GoldenDict code
was copied, ported, or consulted while writing Golddig. The name is a tribute.

## Dictionary packs

Dictionary packs are **not** covered by the MPL. Each pack is a separate distribution of
third-party data under that data's own licence, recorded in the pack's `manifest` and
`sources` tables and reproduced here.

Packs are generated artifacts and are not committed to this repository (the one exception
is the tiny authored `packs/vertical-slice.sqlite` test fixture). Build them with
`python scripts/build_all_packs.py`.

### Wiktionary, via Wiktextract and Kaikki.org

Every `kaikki-*` pack is derived from Wiktionary.

- **Upstream:** <https://www.wiktionary.org/>
- **Extraction:** Wiktextract — <https://github.com/tatuylonen/wiktextract>
- **Distribution:** Kaikki.org — <https://kaikki.org/>
- **Licence:** Creative Commons Attribution-ShareAlike 4.0 International
  (<https://creativecommons.org/licenses/by-sa/4.0/>). Wiktionary text is additionally
  available under the GNU Free Documentation License 1.3 or later
  (<https://www.gnu.org/licenses/fdl-1.3.html>).
- **Attribution:** Wiktionary contributors.

Share-alike applies to the packs: redistributing a modified `kaikki-*` pack requires
distributing it under CC BY-SA 4.0 with attribution preserved. It does **not** reach the
Golddig application code, which is an independent work that merely reads the data.

Each generated manifest additionally records the exact dump URL, its HTTP `Last-Modified`
date, and a SHA-256 of the downloaded source, so a pack can be traced to the snapshot it
came from.

### Pronunciation recordings

Where a Wiktionary entry links an audio recording, Golddig stores the URL and plays it only
when the reader clicks it. The files are hosted on Wikimedia Commons and are **licensed
individually** — most under CC BY-SA or CC0, some public domain. Golddig does not
redistribute them; check the file's own page on Commons before reusing one.

### Tatoeba

Where example sentences are imported from Tatoeba (<https://tatoeba.org>):

- **Licence:** CC BY 2.0 FR (<https://creativecommons.org/licenses/by/2.0/fr/>) for the
  main sentence exports, with a separately published CC0 subset.
- **Attribution:** Tatoeba contributors. Golddig records the sentence ID on each imported
  example so individual sentences remain traceable.

## Speech synthesis

The speaker buttons synthesize speech with Microsoft's neural voices through the same
service Microsoft Edge's "Read Aloud" feature uses (`speech.platform.bing.com`). That
service is not a documented public API: the app talks to it the way Edge does, following
the open-source `edge-tts` project (<https://github.com/rany2/edge-tts>, GPL-3.0; Golddig
contains none of its code, only its protocol description), and Microsoft may change or
withdraw it at any time. Each click sends the text to be spoken to Microsoft, so it needs
the network; the app sends nothing else. Ordinary dictionary lookup never leaves the
machine. When the service is unreachable the app falls back to the platform's Web Speech
API (`window.speechSynthesis`) and whatever voices the host provides, and says so beneath
the headword. Golddig bundles no voice data.
