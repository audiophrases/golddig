// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// The text-only half of "right-click a word, look it up": deciding what the reader meant
// to look up and how to describe it. No DOM and no Tauri here, so all of it is unit-tested;
// the event handling lives in contextMenu.ts.

import type { SearchSuggestion } from './types';

/** Longest text a right-click may hand to the search box. A dragged paragraph is not a query. */
export const MAX_LOOKUP_CHARS = 200;

/** Longest text shown inside the menu label before it is shortened. */
const MAX_LABEL_CHARS = 40;

/**
 * Punctuation, symbols and whitespace to strip from the ends of a selection. A drag that
 * catches the comma after a word, or the quotation marks around it, still means the word.
 * The hyphen-minus is deliberately kept: `-ing` and `pre-` are real headwords (affixes).
 */
const EDGE_JUNK = /^(?:(?!-)[\s\p{P}\p{S}])+|(?:(?!-)[\s\p{P}\p{S}])+$/gu;

/** Turns raw selected text into a query: collapsed whitespace, trimmed edges, bounded length. */
export function normalizeLookupText(raw: string): string {
  const collapsed = raw.replace(/\s+/g, ' ').trim();
  const stripped = collapsed.replace(EDGE_JUNK, '');
  return Array.from(stripped).slice(0, MAX_LOOKUP_CHARS).join('').trim();
}

export interface WordHit {
  word: string;
  /** Offsets into the text the hit was found in, so the caller can select it. */
  start: number;
  end: number;
}

let segmenter: Intl.Segmenter | null | undefined;

function wordSegmenter(): Intl.Segmenter | null {
  if (segmenter !== undefined) return segmenter;
  segmenter =
    typeof Intl !== 'undefined' && 'Segmenter' in Intl
      ? new Intl.Segmenter(undefined, { granularity: 'word' })
      : null;
  return segmenter;
}

/** Fallback word boundary when `Intl.Segmenter` is missing: letters, marks, digits and the
 *  in-word punctuation of the pack languages (apostrophes, the Catalan middle dot). */
const WORD_CHAR = /[\p{L}\p{M}\p{N}'’·]/u;

/**
 * The word containing `offset` in `text`, or ending exactly at it, or null when the offset
 * is on whitespace or punctuation. Uses `Intl.Segmenter`, which understands that `l'home`
 * and `col·lecció` are single words and can split Chinese text without spaces.
 */
export function wordAt(text: string, offset: number, useSegmenter = true): WordHit | null {
  if (offset < 0 || offset > text.length) return null;
  const seg = useSegmenter ? wordSegmenter() : null;
  if (seg) {
    let ending: WordHit | null = null;
    for (const s of seg.segment(text)) {
      if (!s.isWordLike) continue;
      const end = s.index + s.segment.length;
      if (s.index <= offset && offset < end) {
        return { word: s.segment, start: s.index, end };
      }
      if (end === offset) ending = { word: s.segment, start: s.index, end };
      if (s.index > offset) break;
    }
    return ending;
  }
  // Scan outwards from the offset. Treat a caret just after a word as being on it.
  let start = offset;
  let end = offset;
  if (!(start < text.length && WORD_CHAR.test(text[start]))) {
    if (start > 0 && WORD_CHAR.test(text[start - 1])) {
      start -= 1;
      end = start;
    } else {
      return null;
    }
  }
  while (start > 0 && WORD_CHAR.test(text[start - 1])) start--;
  while (end < text.length && WORD_CHAR.test(text[end])) end++;
  return { word: text.slice(start, end), start, end };
}

/**
 * Menu label for looking `text` up. Long selections are shortened for the label only —
 * the whole text is still what gets searched.
 */
export function lookupLabel(text: string): string {
  const chars = Array.from(text);
  const shown = chars.length > MAX_LABEL_CHARS ? `${chars.slice(0, MAX_LABEL_CHARS - 1).join('')}…` : text;
  return `Look up “${shown}”`;
}

/**
 * Escapes text for a native menu label. On Windows a bare `&` marks the next letter as the
 * keyboard mnemonic and is not drawn, so `R&D` would show as `RD`; `&&` renders one `&` on
 * every platform Tauri's menu crate supports.
 */
export function escapeMenuLabel(text: string): string {
  return text.replace(/&/g, '&&');
}

/**
 * Which hit to open for an explicit look-up. Ranking already sorted the candidates; among
 * those tied at the top (same match kind and matched spelling as the first), prefer the one
 * in `preferLanguage` — a word right-clicked inside a Catalan entry is most likely Catalan.
 * Returns the index into `results`, or -1 when there is nothing to open.
 */
export function preferredHitIndex(results: SearchSuggestion[], preferLanguage?: string): number {
  if (!results.length) return -1;
  if (!preferLanguage) return 0;
  const top = results[0];
  for (let i = 0; i < results.length; i++) {
    const r = results[i];
    if (r.match_type !== top.match_type || r.matched_term !== top.matched_term) break;
    if (r.language === preferLanguage) return i;
  }
  return 0;
}
