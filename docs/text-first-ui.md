# Text-first interface direction

## Product statement

Golddig is a **reading and lookup tool**, not a media browser. Its visual quality comes from typography, hierarchy, spacing, and fast interaction. The primary result is formatted text.

## Result layout

Use a restrained single reading column with:

1. a persistent search field;
2. lemma, language, part of speech, IPA, and an optional compact audio button;
3. concise translations near the top;
4. numbered definitions;
5. indented examples and their translations;
6. synonyms, related words, collocations, and forms as compact text lists;
7. subtle source separators and expandable attribution details.

Results from different dictionaries may share a page, but each source remains identifiable. Prefer typographic section headings and thin rules over nested card chrome.

## Allowed presentation vocabulary

- semantic headings and paragraphs;
- numbered and bulleted lists;
- bold, italics, small caps, IPA, superscripts, and language labels;
- restrained color for labels, links, regional markers, and source identity;
- inline icons only when their meaning is obvious, such as pronunciation playback or expand/collapse;
- generous but not wasteful whitespace;
- light and dark themes with accessible contrast.

## Avoid

- entry illustrations, thumbnails, banners, gradients, and decorative photography;
- arbitrary source HTML, CSS, JavaScript, tracking pixels, or remote fonts;
- large rounded cards, deep shadows, animated dashboards, and ornamental transitions;
- displaying the same metadata in several badges when plain text is clearer;
- hiding definitions behind tabs merely to make the page look sparse;
- downloading audio or other media before the user asks for it;
- layout shifts while results are loading.

Images from imported dictionaries are out of the initial product surface. If image support is ever added, it should be an explicit optional attachment rather than part of ordinary lookup.

## Rendering model

The Rust core returns typed lexical data, not ready-made source HTML. The Svelte layer renders a small semantic component set such as:

- `EntryHeader`
- `TranslationLine`
- `DefinitionList`
- `ExampleBlock`
- `RelationList`
- `CollocationList`
- `SourceAttribution`

Legacy markup should be parsed into a safe intermediate representation containing only supported text structures and links. Unsupported styling and media are discarded or exposed as a clearly labeled optional attachment. One Golddig stylesheet controls the final appearance.

## Performance behavior

- Render the best exact result and its first source immediately.
- Add remaining source sections progressively without changing the reading position.
- Collapse exceptionally long sources after a useful preview.
- Keep suggestions as plain virtualized text rows when lists are large.
- Avoid mounting hidden tab contents.
- Cache parsed result DTOs and prepared SQLite statements, not generated HTML.
- Never block first text paint on audio, update checks, provenance expansion, or full-text indexing.
- Benchmark both data-query time and time-to-visible-text with small and pathological entries.

## Keyboard and accessibility

- Focus the search box on launch and with `Ctrl/Cmd+L`.
- Support arrow-key suggestion navigation and Enter to open.
- Provide shortcuts to move between source sections and return to the result heading.
- Use selectable text and native copy behavior.
- Use semantic HTML landmarks and headings, visible focus states, and screen-reader labels.
- Respect reduced-motion and system font-size preferences.

## Initial visual acceptance criteria

- A user can scan lemma, pronunciation, main translation, and first definition without scrolling on a typical laptop window.
- The interface remains clear in grayscale and does not depend on icons or color alone.
- No image, network request, animation, or custom source stylesheet is needed to render a complete dictionary result.
- A long multi-source entry remains responsive while selecting, copying, collapsing, and navigating text.
