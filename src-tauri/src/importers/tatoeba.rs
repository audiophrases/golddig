// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::ExampleRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TatoebaTranslationItem {
    pub id: i64,
    pub text: String,
    pub lang: String,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TatoebaSentenceRecord {
    pub id: i64,
    pub text: String,
    pub lang: String,
    pub license: Option<String>,
    #[serde(default)]
    pub translations: Vec<Vec<TatoebaTranslationItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BilingualSentencePair {
    pub source_id: i64,
    pub source_lang: String,
    pub source_text: String,
    pub target_id: i64,
    pub target_lang: String,
    pub target_text: String,
    pub license: String,
    pub attribution: String,
}

impl TatoebaSentenceRecord {
    /// Extracts flattened bilingual sentence pairs from this record.
    pub fn into_pairs(self) -> Vec<BilingualSentencePair> {
        let mut pairs = Vec::new();
        let src_license = self
            .license
            .as_deref()
            .unwrap_or("CC BY 2.0 FR")
            .to_string();

        for group in self.translations {
            for trans in group {
                let target_license = trans.license.as_deref().unwrap_or(&src_license).to_string();

                pairs.push(BilingualSentencePair {
                    source_id: self.id,
                    source_lang: self.lang.clone(),
                    source_text: self.text.clone(),
                    target_id: trans.id,
                    target_lang: trans.lang,
                    target_text: trans.text,
                    license: target_license,
                    attribution: format!("Tatoeba.org #{} -> #{}", self.id, trans.id),
                });
            }
        }
        pairs
    }

    /// Converts into a Golddig ExampleRecord suitable for attaching to dictionary senses.
    pub fn to_example_record(&self, target_lang: Option<&str>) -> Option<ExampleRecord> {
        let mut best_translation = None;
        for group in &self.translations {
            for trans in group {
                if let Some(target) = target_lang {
                    if trans.lang == target {
                        best_translation = Some(trans.text.clone());
                        break;
                    }
                } else if best_translation.is_none() {
                    best_translation = Some(trans.text.clone());
                }
            }
            if best_translation.is_some() && target_lang.is_some() {
                break;
            }
        }

        Some(ExampleRecord {
            text: self.text.clone(),
            translation: best_translation,
            roman: None,
            source_id: format!("tatoeba:{}", self.id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn test_parse_real_tatoeba_fixtures() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let path = root.join("fixtures/tatoeba-samples.json");
        let file = File::open(path).expect("fixtures/tatoeba-samples.json must exist");
        let records: Vec<TatoebaSentenceRecord> =
            serde_json::from_reader(file).expect("JSON must parse");

        assert!(!records.is_empty(), "Must load sample records");

        let mut total_pairs = 0;
        for r in records {
            let pairs = r.into_pairs();
            total_pairs += pairs.len();
        }

        assert!(total_pairs > 10, "Must extract bilingual sentence pairs");
    }

    #[test]
    fn test_convert_to_example_record() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let path = root.join("fixtures/tatoeba-samples.json");
        let file = File::open(path).expect("fixtures/tatoeba-samples.json must exist");
        let records: Vec<TatoebaSentenceRecord> =
            serde_json::from_reader(file).expect("JSON must parse");

        let first = &records[0];
        let ex = first.to_example_record(None).unwrap();
        assert_eq!(ex.text, first.text);
        assert!(!ex.source_id.is_empty());
    }
}
