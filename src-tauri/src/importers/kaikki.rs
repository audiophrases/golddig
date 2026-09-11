// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{
    EntryRecord, ExampleRecord, FormRecord, PronunciationRecord, SenseRecord, TranslationRecord,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiSound {
    pub ipa: Option<String>,
    pub tags: Option<Vec<String>>,
    pub audio: Option<String>,
    pub ogg_url: Option<String>,
    pub mp3_url: Option<String>,
    /// Wiktextract emits the Chinese romanization under the hyphenated key `zh-pron`.
    /// The alias keeps older snake_case dumps working.
    #[serde(rename = "zh-pron", alias = "zh_pron")]
    pub zh_pron: Option<String>,
    /// Some editions label the romanization generically instead.
    pub roman: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiExample {
    pub text: Option<String>,
    pub translation: Option<String>,
    pub english: Option<String>,
    pub roman: Option<String>,
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
    pub roman: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaikkiSense {
    pub id: Option<String>,
    pub glosses: Option<Vec<String>>,
    pub examples: Option<Vec<KaikkiExample>>,
    pub translations: Option<Vec<KaikkiTranslation>>,
    pub synonyms: Option<Vec<KaikkiSynonym>>,
    pub related: Option<Vec<KaikkiSynonym>>,
    pub derived: Option<Vec<KaikkiSynonym>>,
    pub coordinate_terms: Option<Vec<KaikkiSynonym>>,
    pub tags: Option<Vec<String>>,
    pub raw_tags: Option<Vec<String>>,
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
    pub related: Option<Vec<KaikkiSynonym>>,
    pub derived: Option<Vec<KaikkiSynonym>>,
    pub coordinate_terms: Option<Vec<KaikkiSynonym>>,
    pub synonyms: Option<Vec<KaikkiSynonym>>,
    /// Wiktextract places translations at entry level as well as per sense. Dropping
    /// this array was why shipped packs had 0% translation coverage.
    pub translations: Option<Vec<KaikkiTranslation>>,
}

/// Flattens a Wiktextract relation list into plain strings. Where a romanization is
/// present it is appended so non-Latin scripts stay readable, e.g. `اليوم (el-yūma)`.
fn collect_related(list: &Option<Vec<KaikkiSynonym>>, out: &mut Vec<String>) {
    if let Some(items) = list {
        for item in items {
            if let Some(word) = item.word.as_deref().map(clean_relation_word) {
                if word.is_empty() {
                    continue;
                }
                match item.roman.as_deref().map(str::trim) {
                    Some(r) if !r.is_empty() && r != word => out.push(format!("{} ({})", word, r)),
                    _ => out.push(word),
                }
            }
        }
    }
}

/// Trims the dangling bracket Wiktextract leaves behind when it lifts a parenthetical into
/// `tags` or `roman` but keeps the opening bracket on the word — the Chinese extract emits
/// `"您好 ("`, which would otherwise render as `您好 ( (nín hǎo)`.
fn clean_relation_word(word: &str) -> String {
    word.trim()
        .trim_end_matches(|c: char| c == '(' || c == '（' || c == ',' || c.is_whitespace())
        .trim()
        .to_string()
}

/// Maps a Wiktextract translation list into TranslationRecords, skipping entries with
/// no resolvable target language so `und` rows never reach the UI.
fn collect_translations(
    list: &Option<Vec<KaikkiTranslation>>,
    source_id: &str,
    out: &mut Vec<TranslationRecord>,
) {
    if let Some(items) = list {
        for tr in items {
            let word = match tr.word.as_deref().map(str::trim) {
                Some(w) if !w.is_empty() => w,
                _ => continue,
            };
            let target = tr
                .code
                .as_deref()
                .or(tr.lang_code.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(target) = target else { continue };
            out.push(TranslationRecord {
                target_lang: target.to_string(),
                text: word.to_string(),
                source_id: source_id.to_string(),
            });
        }
    }
}

fn dedupe_preserving_order(items: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| seen.insert(item.clone()));
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
            "en" => "en".to_string(),
            "ary" => "ary".to_string(),
            "zh" | "cmn" => "zh".to_string(),
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

        // Map pronunciations (IPA plus romanized / pinyin transcriptions), keeping the
        // region tags and recording URLs the source provides.
        let mut pronunciations = Vec::new();
        if let Some(sounds) = &self.sounds {
            for sound in sounds {
                let tags = sound.tags.clone().unwrap_or_default();
                let audio = sound
                    .mp3_url
                    .clone()
                    .or_else(|| sound.ogg_url.clone())
                    .or_else(|| sound.audio.clone());

                if let Some(ipa) = &sound.ipa {
                    let clean_ipa = ipa.trim().to_string();
                    if !clean_ipa.is_empty() {
                        pronunciations.push(PronunciationRecord {
                            ipa: clean_ipa,
                            kind: "ipa".to_string(),
                            tags: tags.clone(),
                            audio: audio.clone(),
                            source_id: source_id.to_string(),
                        });
                    }
                }
                // `zh-pron` for Chinese; `roman` is the generic romanization channel.
                let romanization = sound
                    .zh_pron
                    .as_deref()
                    .or(sound.roman.as_deref())
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                if let Some(rom) = romanization {
                    pronunciations.push(PronunciationRecord {
                        ipa: rom.to_string(),
                        kind: "transcription".to_string(),
                        tags: tags.clone(),
                        audio: audio.clone(),
                        source_id: source_id.to_string(),
                    });
                }
                // An audio-only sound entry still carries a usable recording.
                if sound.ipa.is_none() && romanization.is_none() {
                    if let Some(url) = audio {
                        pronunciations.push(PronunciationRecord {
                            ipa: String::new(),
                            kind: "audio".to_string(),
                            tags,
                            audio: Some(url),
                            source_id: source_id.to_string(),
                        });
                    }
                }
            }
        }

        // For English, surface the American pronunciation first — the app advertises AmE.
        if language == "en" {
            pronunciations.sort_by_key(|p| {
                let us = p.tags.iter().any(|t| {
                    let t = t.to_ascii_lowercase();
                    t.contains("us") || t.contains("american") || t.contains("ga")
                });
                if us {
                    0
                } else {
                    1
                }
            });
        }

        // Extract related / derived / coordinate terms at entry level. These are
        // Wiktionary relations, not corpus statistics — see collect_related.
        let mut entry_collocations = Vec::new();
        collect_related(&self.related, &mut entry_collocations);
        collect_related(&self.derived, &mut entry_collocations);
        collect_related(&self.coordinate_terms, &mut entry_collocations);
        dedupe_preserving_order(&mut entry_collocations);

        let mut entry_synonyms = Vec::new();
        collect_related(&self.synonyms, &mut entry_synonyms);

        // Entry-level translations. Wiktextract puts translations here as well as on
        // each sense; reading only the per-sense field lost most of them.
        let mut entry_translations = Vec::new();
        collect_translations(&self.translations, source_id, &mut entry_translations);

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
                            if text.trim().is_empty() {
                                continue;
                            }
                            let trans = ex.translation.or(ex.english);
                            examples.push(ExampleRecord {
                                text,
                                translation: trans,
                                roman: ex.roman,
                                source_id: source_id.to_string(),
                            });
                        }
                    }
                }

                let mut translations = Vec::new();
                collect_translations(&raw_sense.translations, source_id, &mut translations);

                let mut synonyms = Vec::new();
                collect_related(&raw_sense.synonyms, &mut synonyms);

                // Sense-level relations belong to this sense; entry-level ones are
                // attached to the first sense since the source does not scope them.
                let mut collocations = Vec::new();
                collect_related(&raw_sense.related, &mut collocations);
                collect_related(&raw_sense.derived, &mut collocations);
                collect_related(&raw_sense.coordinate_terms, &mut collocations);

                if idx == 0 {
                    collocations.extend(entry_collocations.iter().cloned());
                    synonyms.extend(entry_synonyms.iter().cloned());
                    translations.extend(entry_translations.iter().cloned());
                }
                dedupe_preserving_order(&mut collocations);
                dedupe_preserving_order(&mut synonyms);

                let mut tags = raw_sense.tags.unwrap_or_default();
                tags.extend(raw_sense.raw_tags.unwrap_or_default());
                dedupe_preserving_order(&mut tags);

                senses.push(SenseRecord {
                    id: sense_id,
                    definition,
                    examples,
                    translations,
                    synonyms,
                    collocations,
                    tags,
                    source_id: source_id.to_string(),
                });
            }
        }

        if senses.is_empty() {
            return None;
        }

        // Map forms and romanizations/transcriptions
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
                    assert_eq!(record.language, "en");
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
