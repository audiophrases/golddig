use crate::model::{
    EntryRecord, ExampleRecord, FormRecord, PronunciationRecord, SenseRecord, TranslationRecord,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiSound {
    pub ipa: Option<String>,
    pub tags: Option<Vec<String>>,
    pub audio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiExample {
    pub text: Option<String>,
    pub translation: Option<String>,
    pub english: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiTranslation {
    pub lang: Option<String>,
    pub code: Option<String>,
    pub lang_code: Option<String>,
    pub word: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiSynonym {
    pub word: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiSense {
    pub id: Option<String>,
    pub glosses: Option<Vec<String>>,
    pub examples: Option<Vec<KaikkiExample>>,
    pub translations: Option<Vec<KaikkiTranslation>>,
    pub synonyms: Option<Vec<KaikkiSynonym>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiForm {
    pub form: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiEntry {
    pub word: String,
    pub lang: Option<String>,
    pub lang_code: Option<String>,
    pub pos: Option<String>,
    pub etymology_number: Option<serde_json::Value>,
    pub sounds: Option<Vec<KaikkiSound>>,
    pub senses: Option<Vec<KaikkiSense>>,
    pub forms: Option<Vec<KaikkiForm>>,
}

impl KaikkiEntry {
    /// Maps a raw Kaikki Wiktextract entry into Golddig's normalized EntryRecord.
    pub fn into_entry_record(self, source_id: &str) -> Option<EntryRecord> {
        let lemma = self.word.trim().to_string();
        if lemma.is_empty() {
            return None;
        }

        let raw_lang = self.lang_code.unwrap_or_else(|| "und".to_string());
        let language = match raw_lang.as_str() {
            "en" => "en-US".to_string(),
            other => other.to_string(),
        };

        let etym_suffix = match &self.etymology_number {
            Some(serde_json::Value::String(s)) => format!(":e{}", s),
            Some(serde_json::Value::Number(n)) => format!(":e{}", n),
            _ => String::new(),
        };

        let entry_id = match &self.pos {
            Some(p) => format!("{}:{}:{}{}", language, p, lemma, etym_suffix),
            None => format!("{}:{}{}", language, lemma, etym_suffix),
        };

        // Map pronunciations (IPA)
        let mut pronunciations = Vec::new();
        if let Some(sounds) = self.sounds {
            for sound in sounds {
                if let Some(ipa) = sound.ipa {
                    let clean_ipa = ipa.trim().to_string();
                    if !clean_ipa.is_empty() {
                        pronunciations.push(PronunciationRecord {
                            ipa: clean_ipa,
                            kind: "ipa".to_string(),
                            source_id: source_id.to_string(),
                        });
                    }
                }
            }
        }

        // Map senses, definitions, translations, examples, synonyms
        let mut senses = Vec::new();
        if let Some(raw_senses) = self.senses {
            for (idx, raw_sense) in raw_senses.into_iter().enumerate() {
                let definition = if let Some(glosses) = raw_sense.glosses {
                    glosses.join("; ")
                } else {
                    continue;
                };

                if definition.trim().is_empty() {
                    continue;
                }

                let sense_id = raw_sense
                    .id
                    .unwrap_or_else(|| format!("{}:s{}", entry_id, idx + 1));

                let mut examples = Vec::new();
                if let Some(raw_examples) = raw_sense.examples {
                    for ex in raw_examples {
                        if let Some(text) = ex.text {
                            let trans = ex.translation.or(ex.english);
                            examples.push(ExampleRecord {
                                text,
                                translation: trans,
                                source_id: source_id.to_string(),
                            });
                        }
                    }
                }

                let mut translations = Vec::new();
                if let Some(raw_tr) = raw_sense.translations {
                    for tr in raw_tr {
                        if let Some(tr_word) = tr.word {
                            let target = tr
                                .code
                                .or(tr.lang_code)
                                .unwrap_or_else(|| "und".to_string());
                            translations.push(TranslationRecord {
                                target_lang: target,
                                text: tr_word,
                                source_id: source_id.to_string(),
                            });
                        }
                    }
                }

                let mut synonyms = Vec::new();
                if let Some(raw_syn) = raw_sense.synonyms {
                    for s in raw_syn {
                        if let Some(w) = s.word {
                            synonyms.push(w);
                        }
                    }
                }

                senses.push(SenseRecord {
                    id: sense_id,
                    definition,
                    examples,
                    translations,
                    synonyms,
                    collocations: Vec::new(),
                    source_id: source_id.to_string(),
                });
            }
        }

        if senses.is_empty() {
            return None;
        }

        // Map forms
        let mut forms = Vec::new();
        if let Some(raw_forms) = self.forms {
            for f in raw_forms {
                if let Some(form_str) = f.form {
                    let kind = f
                        .tags
                        .map(|t| t.join(", "))
                        .unwrap_or_else(|| "form".to_string());
                    forms.push(FormRecord {
                        form: form_str,
                        kind,
                        source_id: source_id.to_string(),
                    });
                }
            }
        }

        Some(EntryRecord {
            id: entry_id,
            language,
            lemma,
            pos: self.pos,
            pronunciations,
            senses,
            forms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;

    #[test]
    fn test_parse_real_kaikki_catalan_sample() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let path = root.join("fixtures/kaikki-sample-ca.jsonl");
        let file = File::open(path).expect("kaikki-sample-ca.jsonl must exist");
        let reader = BufReader::new(file);

        let mut converted = 0;
        for line in reader.lines() {
            let l = line.unwrap();
            if let Ok(entry) = serde_json::from_str::<KaikkiEntry>(&l) {
                if let Some(record) = entry.into_entry_record("kaikki-wiktionary") {
                    assert!(!record.lemma.is_empty());
                    assert_eq!(record.language, "ca");
                    assert!(!record.senses.is_empty());
                    converted += 1;
                }
            }
        }
        assert!(
            converted > 0,
            "Must successfully convert Catalan Kaikki entries"
        );
    }

    #[test]
    fn test_parse_real_kaikki_english_sample() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let path = root.join("fixtures/kaikki-sample-en.jsonl");
        let file = File::open(path).expect("kaikki-sample-en.jsonl must exist");
        let reader = BufReader::new(file);

        let mut converted = 0;
        let mut has_translations = false;
        for line in reader.lines() {
            let l = line.unwrap();
            if let Ok(entry) = serde_json::from_str::<KaikkiEntry>(&l) {
                if let Some(record) = entry.into_entry_record("kaikki-wiktionary") {
                    assert!(!record.lemma.is_empty());
                    assert_eq!(record.language, "en-US");
                    for s in &record.senses {
                        if !s.translations.is_empty() {
                            has_translations = true;
                        }
                    }
                    converted += 1;
                }
            }
        }
        assert!(converted >= 5, "Must convert multiple English entries");
        assert!(
            has_translations,
            "English senses should parse Wiktionary translations"
        );
    }
}
