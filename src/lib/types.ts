// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

export interface SourceMeta {
  id: string;
  name: string;
  url: string;
  license: string;
  attribution: string;
}

export interface PackInfo {
  id: string;
  name: string;
  version: string;
  languages: string[];
  path: string;
  enabled: boolean;
  entry_count: number;
  /** Lower sorts first. Breaks ranking ties between packs; the reader sets it. */
  priority: number;
}

export interface PronunciationRecord {
  ipa: string;
  type: string;
  /** Region / register labels from the source, e.g. ["US"]. */
  tags?: string[];
  /** Upstream recording URL, when the source provides one. */
  audio?: string | null;
  source_id: string;
}

export interface ExampleRecord {
  text: string;
  translation?: string | null;
  /** Latin transliteration of `text`, for Arabic and Chinese. */
  roman?: string | null;
  source_id: string;
}

export interface TranslationRecord {
  target_lang: string;
  text: string;
  source_id: string;
}

export interface SenseRecord {
  id: string;
  definition: string;
  examples?: ExampleRecord[];
  translations?: TranslationRecord[];
  synonyms?: string[];
  collocations?: string[];
  /** Register / domain labels, e.g. ["informal"]. */
  tags?: string[];
  source_id: string;
}

export interface FormRecord {
  form: string;
  type: string;
  source_id: string;
}

export interface EntryRecord {
  id: string;
  language: string;
  lemma: string;
  pos?: string | null;
  pronunciations: PronunciationRecord[];
  senses: SenseRecord[];
  forms: FormRecord[];
}

export interface SearchSuggestion {
  entry_id: string;
  lemma: string;
  language: string;
  pos?: string | null;
  matched_term: string;
  match_type: string;
}

/** A pack that could not be opened, surfaced so an empty dictionary is never silent. */
export interface PackLoadError {
  path: string;
  message: string;
}

/** Where packs were looked for and what happened, for the status bar. */
export interface PackDiagnostics {
  pack_dir: string;
  loaded: number;
  errors: PackLoadError[];
  /** Where pack order and enabled flags are saved; empty means they are session-only. */
  settings_path: string;
}
