// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, it, expect } from 'vitest';
import {
  escapeMenuLabel,
  lookupLabel,
  normalizeLookupText,
  preferredHitIndex,
  wordAt,
  MAX_LOOKUP_CHARS,
} from '../src/lib/lookup';
import type { SearchSuggestion } from '../src/lib/types';

describe('normalizeLookupText', () => {
  it('collapses whitespace and trims', () => {
    expect(normalizeLookupText('  bon \n  dia  ')).toBe('bon dia');
  });

  it('strips the punctuation a drag selection catches, on both ends', () => {
    expect(normalizeLookupText('“gat”,')).toBe('gat');
    expect(normalizeLookupText('¿qué?')).toBe('qué');
    expect(normalizeLookupText('(hola) —')).toBe('hola');
  });

  it('keeps in-word punctuation and leading or trailing hyphens', () => {
    expect(normalizeLookupText("l'home")).toBe("l'home");
    expect(normalizeLookupText('col·lecció')).toBe('col·lecció');
    expect(normalizeLookupText('-ing')).toBe('-ing');
    expect(normalizeLookupText('pre-')).toBe('pre-');
  });

  it('bounds the length by characters, not UTF-16 units', () => {
    const long = '猫'.repeat(MAX_LOOKUP_CHARS + 50);
    expect(Array.from(normalizeLookupText(long)).length).toBe(MAX_LOOKUP_CHARS);
    expect(normalizeLookupText('')).toBe('');
    expect(normalizeLookupText('...')).toBe('');
  });
});

describe('wordAt', () => {
  const text = "El gat i l'home van a col·lecció.";

  it('finds the word containing the offset', () => {
    expect(wordAt(text, 4)).toEqual({ word: 'gat', start: 3, end: 6 });
    expect(wordAt(text, 3)).toEqual({ word: 'gat', start: 3, end: 6 });
  });

  it('treats a caret just after a word as on it', () => {
    expect(wordAt(text, 6)?.word).toBe('gat');
  });

  it('returns null on whitespace between words and on punctuation', () => {
    expect(wordAt('a  b', 2)).toBeNull();
    expect(wordAt(text, text.length)?.word).not.toBe('.');
    expect(wordAt('. ,', 1)).toBeNull();
  });

  it('keeps apostrophes and the Catalan middle dot inside words', () => {
    expect(wordAt(text, 10)?.word).toBe("l'home");
    expect(wordAt(text, 25)?.word).toBe('col·lecció');
  });

  it('splits Chinese text without spaces', () => {
    const hit = wordAt('我喜欢猫和狗', 3);
    expect(hit).not.toBeNull();
    expect(hit!.word.length).toBeLessThan(6);
    expect(hit!.word).toContain('猫');
  });

  it('rejects offsets outside the text', () => {
    expect(wordAt(text, -1)).toBeNull();
    expect(wordAt(text, text.length + 1)).toBeNull();
  });

  it('agrees with the fallback scanner on ordinary words', () => {
    for (const offset of [0, 4, 6, 10, 25]) {
      expect(wordAt(text, offset, false)?.word).toBe(wordAt(text, offset)?.word);
    }
    expect(wordAt('a  b', 2, false)).toBeNull();
  });
});

describe('lookupLabel', () => {
  it('quotes the text', () => {
    expect(lookupLabel('gat')).toBe('Look up “gat”');
  });

  it('shortens long selections for display only', () => {
    const label = lookupLabel('a'.repeat(100));
    expect(label.endsWith('…”')).toBe(true);
    expect(label.length).toBeLessThan(60);
  });
});

describe('escapeMenuLabel', () => {
  it('doubles ampersands so Windows does not eat them as mnemonics', () => {
    expect(escapeMenuLabel('R&D')).toBe('R&&D');
    expect(escapeMenuLabel('gat')).toBe('gat');
  });
});

describe('preferredHitIndex', () => {
  const hit = (language: string, match_type = 'lemma', matched_term = 'gat'): SearchSuggestion => ({
    entry_id: `${language}:${matched_term}`,
    lemma: matched_term,
    language,
    matched_term,
    match_type,
  });

  it('opens nothing for no results', () => {
    expect(preferredHitIndex([], 'ca')).toBe(-1);
  });

  it('prefers the entry in the current language among tied top hits', () => {
    const results = [hit('es'), hit('ca'), hit('fr')];
    expect(preferredHitIndex(results, 'ca')).toBe(1);
  });

  it('never reaches past the tie into a weaker match', () => {
    const results = [hit('es'), hit('ca', 'form')];
    expect(preferredHitIndex(results, 'ca')).toBe(0);
    const results2 = [hit('es'), hit('ca', 'lemma', 'gato')];
    expect(preferredHitIndex(results2, 'ca')).toBe(0);
  });

  it('falls back to the top hit with no preference or no match', () => {
    const results = [hit('es'), hit('fr')];
    expect(preferredHitIndex(results)).toBe(0);
    expect(preferredHitIndex(results, 'ca')).toBe(0);
  });
});
