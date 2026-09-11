export interface SourceMeta {
  id: string;
  name: string;
  url: string;
  license: string;
  attribution: string;
}

export interface PronunciationRecord {
  ipa: string;
  type: string;
  source_id: string;
}

export interface ExampleRecord {
  text: string;
  translation?: string | null;
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
