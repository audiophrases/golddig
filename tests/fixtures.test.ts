import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, it, expect } from 'vitest';

describe('Fixtures and Schema Validation', () => {
  const manifestPath = resolve(process.cwd(), 'fixtures/vertical-slice.manifest.json');
  const entriesPath = resolve(process.cwd(), 'fixtures/vertical-slice.entries.jsonl');

  it('validates manifest structure and required 5 languages', () => {
    const raw = readFileSync(manifestPath, 'utf-8');
    const manifest = JSON.parse(raw);

    expect(manifest.id).toBeDefined();
    expect(manifest.languages).toEqual(expect.arrayContaining(['en-US', 'ca', 'fr', 'de', 'es']));
    expect(manifest.sources.length).toBeGreaterThanOrEqual(1);

    for (const src of manifest.sources) {
      expect(src.id).toBeDefined();
      expect(src.name).toBeDefined();
      expect(src.license).toBeDefined();
      expect(src.attribution).toBeDefined();
    }
  });

  it('validates that fixture entries cover all target languages with full provenance', () => {
    const lines = readFileSync(entriesPath, 'utf-8')
      .split('\n')
      .map(l => l.trim())
      .filter(Boolean);

    expect(lines.length).toBeGreaterThanOrEqual(5);

    const languagesFound = new Set<string>();
    const lemmas = new Set<string>();

    for (const line of lines) {
      const entry = JSON.parse(line);
      expect(entry.id).toBeDefined();
      expect(entry.language).toBeDefined();
      expect(entry.lemma).toBeDefined();
      expect(entry.senses.length).toBeGreaterThanOrEqual(1);

      languagesFound.add(entry.language);
      lemmas.add(entry.lemma);

      for (const sense of entry.senses) {
        expect(sense.definition).toBeDefined();
        expect(sense.source_id).toBeDefined();
        for (const tr of sense.translations || []) {
          expect(tr.target_lang).toBeDefined();
          expect(tr.text).toBeDefined();
          expect(tr.source_id).toBeDefined();
        }
        for (const ex of sense.examples || []) {
          expect(ex.text).toBeDefined();
          expect(ex.source_id).toBeDefined();
        }
      }

      for (const form of entry.forms || []) {
        expect(form.form).toBeDefined();
        expect(form.source_id).toBeDefined();
      }
    }

    expect(languagesFound.has('en-US')).toBe(true);
    expect(languagesFound.has('ca')).toBe(true);
    expect(languagesFound.has('es')).toBe(true);
    expect(languagesFound.has('fr')).toBe(true);
    expect(languagesFound.has('de')).toBe(true);

    // Collision check lemmas are present
    expect(lemmas.has('ano')).toBe(true);
    expect(lemmas.has('año')).toBe(true);
    expect(lemmas.has('col·lecció')).toBe(true);
    expect(lemmas.has('cœur')).toBe(true);
    expect(lemmas.has('Straße')).toBe(true);
  });
});
