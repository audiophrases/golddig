// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import type { EntryRecord, PackInfo, SearchSuggestion } from './types';

// Fixture data used when running in a standalone web browser (e.g. Vite dev/preview)
// without the native Tauri desktop runtime.
export const MOCK_PACKS: PackInfo[] = [
  {
    id: 'vertical-slice',
    name: 'Multi-lingual Core (5 Languages)',
    version: '0.1.0',
    languages: ['en', 'ca', 'es', 'fr', 'de', 'ary', 'zh'],
    path: 'packs/vertical-slice.sqlite',
    enabled: true,
    entry_count: 10,
    priority: 0,
  },
];

export const MOCK_ENTRIES: Record<string, EntryRecord> = {
  'es:hola': {
    id: 'es:hola',
    language: 'es',
    lemma: 'hola',
    pos: 'interjection',
    pronunciations: [{ ipa: '/ˈo.la/', type: 'ipa', source_id: 'wiktionary-core' }],
    senses: [
      {
        id: 'es:hola:s1',
        definition: 'Fórmula habitual de salutación o saludo amistoso al encontrarse con alguien.',
        examples: [
          {
            text: '¡Hola! ¿Cómo estás hoy?',
            translation: 'Hello! How are you today?',
            source_id: 'tatoeba-examples',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'hello', source_id: 'wiktionary-core' },
          { target_lang: 'ca', text: 'hola', source_id: 'wiktionary-core' },
          { target_lang: 'fr', text: 'salut', source_id: 'wiktionary-core' },
        ],
        synonyms: ['saludos', 'buenas'],
        collocations: ['decir hola', 'hola y adiós'],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [],
  },
  'ary:salam': {
    id: 'ary:salam',
    language: 'ary',
    lemma: 'سلام',
    pos: 'interjection',
    pronunciations: [{ ipa: '/sa.laːm/', type: 'ipa', source_id: 'wiktionary-core' }],
    senses: [
      {
        id: 'ary:salam:s1',
        definition: 'تحية د السلام، كيتݣال فالتلاقية والوداع (Peace; hello, hi, greeting used upon meeting or departing).',
        examples: [
          {
            text: 'سلام، كيدير لاباس عليك؟ (Salam, kidayr labas 3lik?)',
            translation: 'Hello, how are you doing?',
            source_id: 'wiktionary-core',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'peace; hello; hi', source_id: 'wiktionary-core' },
          { target_lang: 'es', text: 'hola', source_id: 'wiktionary-core' },
          { target_lang: 'fr', text: 'salut', source_id: 'wiktionary-core' },
        ],
        synonyms: ['أهلا', 'مرحبا'],
        collocations: ['سلام عليكم (salam 3likom)', 'رد السلام'],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [
      { form: 'salam', type: 'romanization', source_id: 'wiktionary-core' },
      { form: 'ssalamu 3likom', type: 'greeting', source_id: 'wiktionary-core' },
    ],
  },
  'zh:nihao': {
    id: 'zh:nihao',
    language: 'zh',
    lemma: '你好',
    pos: 'phrase',
    pronunciations: [
      { ipa: '/ni²¹⁴⁻²¹¹ xɑʊ̯²¹⁴⁻²¹⁽⁴⁾/', type: 'ipa', source_id: 'wiktionary-core' },
      { ipa: 'nǐ hǎo', type: 'pinyin', source_id: 'wiktionary-core' },
    ],
    senses: [
      {
        id: 'zh:nihao:s1',
        definition: '问候语，用于见面时打招呼 (Hello, how do you do; common standard polite greeting).',
        examples: [
          {
            text: '你好，很高兴认识你！ (Nǐ hǎo, hěn gāoxìng rènshì nǐ!)',
            translation: 'Hello, very pleased to meet you!',
            source_id: 'wiktionary-core',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'hello; hi; how do you do', source_id: 'wiktionary-core' },
          { target_lang: 'es', text: 'hola', source_id: 'wiktionary-core' },
          { target_lang: 'ca', text: 'hola', source_id: 'wiktionary-core' },
        ],
        synonyms: ['您好', '哈喽'],
        collocations: ['问你好', '说你好'],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [
      { form: 'nǐhǎo', type: 'pinyin', source_id: 'wiktionary-core' },
      { form: 'nihao', type: 'transcription', source_id: 'wiktionary-core' },
    ],
  },
  'en:light': {
    id: 'en:light',
    language: 'en',
    lemma: 'light',
    pos: 'noun',
    pronunciations: [{ ipa: '/laɪt/', type: 'ipa', source_id: 'wiktionary-core' }],
    senses: [
      {
        id: 'en:light:s1',
        definition: 'Visible electromagnetic radiation that enables sight.',
        examples: [
          {
            text: 'The morning light filtered through the curtains.',
            translation: 'La llum del matí es filtrava per les cortines.',
            source_id: 'tatoeba-examples',
          },
        ],
        translations: [
          { target_lang: 'ca', text: 'llum', source_id: 'wiktionary-core' },
          { target_lang: 'es', text: 'luz', source_id: 'wiktionary-core' },
          { target_lang: 'fr', text: 'lumière', source_id: 'wiktionary-core' },
          { target_lang: 'de', text: 'Licht', source_id: 'wiktionary-core' },
        ],
        synonyms: ['illumination', 'radiance'],
        collocations: ['bright light', 'morning light'],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [{ form: 'lights', type: 'plural', source_id: 'wiktionary-core' }],
  },
  'ca:llum': {
    id: 'ca:llum',
    language: 'ca',
    lemma: 'llum',
    pos: 'noun',
    pronunciations: [{ ipa: '/ˈʎum/', type: 'ipa', source_id: 'softcatala-ca' }],
    senses: [
      {
        id: 'ca:llum:s1',
        definition: "Claror o radiació electromagnètica visible a l'ull humà.",
        examples: [
          {
            text: 'Encén el llum per poder llegir millor.',
            translation: 'Turn on the light so you can read better.',
            source_id: 'softcatala-ca',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'light', source_id: 'softcatala-ca' },
          { target_lang: 'es', text: 'luz', source_id: 'softcatala-ca' },
          { target_lang: 'fr', text: 'lumière', source_id: 'softcatala-ca' },
          { target_lang: 'de', text: 'Licht', source_id: 'softcatala-ca' },
        ],
        synonyms: ['claror', 'resplendor'],
        collocations: ['llum natural', 'fer llum'],
        source_id: 'softcatala-ca',
      },
    ],
    forms: [{ form: 'llums', type: 'plural', source_id: 'softcatala-ca' }],
  },
  'ca:colleccio': {
    id: 'ca:colleccio',
    language: 'ca',
    lemma: 'col·lecció',
    pos: 'noun',
    pronunciations: [{ ipa: '/kuləksiˈo/', type: 'ipa', source_id: 'softcatala-ca' }],
    senses: [
      {
        id: 'ca:colleccio:s1',
        definition: "Conjunt ordenat de coses de la mateixa espècie que tenen interès o valor.",
        examples: [
          {
            text: 'Té una gran col·lecció de llibres antics.',
            translation: 'He has a large collection of antique books.',
            source_id: 'tatoeba-examples',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'collection', source_id: 'softcatala-ca' },
          { target_lang: 'es', text: 'colección', source_id: 'softcatala-ca' },
          { target_lang: 'fr', text: 'collection', source_id: 'softcatala-ca' },
          { target_lang: 'de', text: 'Sammlung', source_id: 'softcatala-ca' },
        ],
        synonyms: ['aplec', 'recull'],
        collocations: ["col·lecció d'art", 'reunir una col·lecció'],
        source_id: 'softcatala-ca',
      },
    ],
    forms: [{ form: 'col·leccions', type: 'plural', source_id: 'softcatala-ca' }],
  },
  'es:ano': {
    id: 'es:ano',
    language: 'es',
    lemma: 'ano',
    pos: 'noun',
    pronunciations: [{ ipa: '/ˈa.no/', type: 'ipa', source_id: 'wiktionary-core' }],
    senses: [
      {
        id: 'es:ano:s1',
        definition: 'Orificio terminal del tubo digestivo a través del cual se expulsan los excrementos.',
        examples: [
          {
            text: 'Estructura anatómica al final del conducto digestivo.',
            translation: null,
            source_id: 'wiktionary-core',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'anus', source_id: 'wiktionary-core' },
          { target_lang: 'ca', text: 'anus', source_id: 'wiktionary-core' },
        ],
        synonyms: [],
        collocations: [],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [{ form: 'anos', type: 'plural', source_id: 'wiktionary-core' }],
  },
  'es:anyo': {
    id: 'es:anyo',
    language: 'es',
    lemma: 'año',
    pos: 'noun',
    pronunciations: [{ ipa: '/ˈa.ɲo/', type: 'ipa', source_id: 'wiktionary-core' }],
    senses: [
      {
        id: 'es:anyo:s1',
        definition: 'Período de doce meses consecutivos que tarda la Tierra en dar una vuelta completa alrededor del Sol.',
        examples: [
          {
            text: 'El año que viene viajaré a Berlín.',
            translation: 'Next year I will travel to Berlin.',
            source_id: 'tatoeba-examples',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'year', source_id: 'wiktionary-core' },
          { target_lang: 'ca', text: 'any', source_id: 'wiktionary-core' },
          { target_lang: 'fr', text: 'année', source_id: 'wiktionary-core' },
          { target_lang: 'de', text: 'Jahr', source_id: 'wiktionary-core' },
        ],
        synonyms: ['doce meses'],
        collocations: ['año bisiesto', 'año nuevo'],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [{ form: 'años', type: 'plural', source_id: 'wiktionary-core' }],
  },
  'fr:coeur': {
    id: 'fr:coeur',
    language: 'fr',
    lemma: 'cœur',
    pos: 'noun',
    pronunciations: [{ ipa: '/kœʁ/', type: 'ipa', source_id: 'wiktionary-core' }],
    senses: [
      {
        id: 'fr:coeur:s1',
        definition: 'Organe musculaire creux assurant la circulation du sang en pompant le liquide dans les vaisseaux.',
        examples: [
          {
            text: 'Son cœur battait la chamade après la course.',
            translation: 'His heart was pounding after the run.',
            source_id: 'tatoeba-examples',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'heart', source_id: 'wiktionary-core' },
          { target_lang: 'ca', text: 'cor', source_id: 'wiktionary-core' },
          { target_lang: 'es', text: 'corazón', source_id: 'wiktionary-core' },
          { target_lang: 'de', text: 'Herz', source_id: 'wiktionary-core' },
        ],
        synonyms: ['organe cardiaque'],
        collocations: ['de tout cœur', 'battement de cœur'],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [{ form: 'cœurs', type: 'plural', source_id: 'wiktionary-core' }],
  },
  'de:strasse': {
    id: 'de:strasse',
    language: 'de',
    lemma: 'Straße',
    pos: 'noun',
    pronunciations: [{ ipa: '/ˈʃtʁaːsə/', type: 'ipa', source_id: 'wiktionary-core' }],
    senses: [
      {
        id: 'de:strasse:s1',
        definition: 'Befestigter Weg für den Verkehr von Fahrzeugen und Fußgängern.',
        examples: [
          {
            text: 'Die Kinder spielen sicher auf der ruhigen Straße.',
            translation: 'The children play safely on the quiet street.',
            source_id: 'tatoeba-examples',
          },
        ],
        translations: [
          { target_lang: 'en', text: 'street', source_id: 'wiktionary-core' },
          { target_lang: 'ca', text: 'carrer', source_id: 'wiktionary-core' },
          { target_lang: 'es', text: 'calle', source_id: 'wiktionary-core' },
          { target_lang: 'fr', text: 'rue', source_id: 'wiktionary-core' },
        ],
        synonyms: ['Weg', 'Gasse'],
        collocations: ['auf der Straße', 'Hauptstraße'],
        source_id: 'wiktionary-core',
      },
    ],
    forms: [{ form: 'Straßen', type: 'plural', source_id: 'wiktionary-core' }],
  },
};

export function mockSuggest(query: string): SearchSuggestion[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const results: SearchSuggestion[] = [];

  for (const [id, entry] of Object.entries(MOCK_ENTRIES)) {
    const lemmaLower = entry.lemma.toLowerCase();
    let isWildcard = q.includes('?') || q.includes('*');
    let isPhrase = q.includes(' ') && !isWildcard;

    let matched = false;

    if (isWildcard) {
      // Regex conversion: ? -> ., * -> .*
      const regexStr = '^' + q.replace(/\?/g, '.').replace(/\*/g, '.*') + '$';
      const re = new RegExp(regexStr, 'i');
      if (re.test(entry.lemma)) {
        results.push({
          entry_id: id,
          lemma: entry.lemma,
          language: entry.language,
          pos: entry.pos,
          matched_term: entry.lemma,
          match_type: 'wildcard',
        });
        matched = true;
      }
    } else if (isPhrase) {
      // Search lemma or examples for phrase
      if (lemmaLower.includes(q)) {
        results.push({
          entry_id: id,
          lemma: entry.lemma,
          language: entry.language,
          pos: entry.pos,
          matched_term: entry.lemma,
          match_type: 'phrase',
        });
        matched = true;
      } else {
        const foundInExamples = entry.senses.some((s) =>
          s.examples?.some((ex) => ex.text.toLowerCase().includes(q) || ex.translation?.toLowerCase().includes(q))
        );
        if (foundInExamples) {
          results.push({
            entry_id: id,
            lemma: entry.lemma,
            language: entry.language,
            pos: entry.pos,
            matched_term: entry.lemma,
            match_type: 'phrase',
          });
          matched = true;
        }
      }
    } else {
      // Check exact lemma match
      if (lemmaLower === q) {
        results.unshift({
          entry_id: id,
          lemma: entry.lemma,
          language: entry.language,
          pos: entry.pos,
          matched_term: entry.lemma,
          match_type: 'lemma',
        });
        matched = true;
      } else if (lemmaLower.startsWith(q)) {
        results.push({
          entry_id: id,
          lemma: entry.lemma,
          language: entry.language,
          pos: entry.pos,
          matched_term: entry.lemma,
          match_type: 'lemma',
        });
        matched = true;
      }
    }

    // Check forms and pronunciations / transcriptions (pinyin / romanization)
    if (!matched && entry.forms) {
      for (const f of entry.forms) {
        if (f.form.toLowerCase().startsWith(q)) {
          results.push({
            entry_id: id,
            lemma: entry.lemma,
            language: entry.language,
            pos: entry.pos,
            matched_term: f.form,
            match_type: 'form',
          });
          matched = true;
          break;
        }
      }
    }

    if (!matched && entry.pronunciations) {
      for (const p of entry.pronunciations) {
        if (p.ipa.toLowerCase().replace(/[\s\d]/g, '').startsWith(q.replace(/[\s\d]/g, ''))) {
          results.push({
            entry_id: id,
            lemma: entry.lemma,
            language: entry.language,
            pos: entry.pos,
            matched_term: p.ipa,
            match_type: 'transcription',
          });
          break;
        }
      }
    }
  }

  return results;
}
