// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub schema_version: i32,
    pub languages: Vec<String>,
    pub created_at: String,
    pub license: String,
    pub sources: Vec<SourceMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMeta {
    pub id: String,
    pub name: String,
    pub url: String,
    pub license: String,
    pub attribution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryRecord {
    pub id: String,
    pub language: String,
    pub lemma: String,
    pub pos: Option<String>,
    #[serde(default)]
    pub pronunciations: Vec<PronunciationRecord>,
    #[serde(default)]
    pub senses: Vec<SenseRecord>,
    #[serde(default)]
    pub forms: Vec<FormRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PronunciationRecord {
    pub ipa: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Region / register labels from the source, e.g. ["US"], ["Received Pronunciation"].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Upstream recording URL, when the source provides one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    /// Empty means "the pack's default source"; the engine fills it in on read. Storing
    /// the same id on every sense, example, form and pronunciation was 12.6% of the JSON.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenseRecord {
    pub id: String,
    pub definition: String,
    #[serde(default)]
    pub examples: Vec<ExampleRecord>,
    #[serde(default)]
    pub translations: Vec<TranslationRecord>,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub collocations: Vec<String>,
    /// Register / domain labels from the source, e.g. ["informal"], ["chemistry"].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Empty means "the pack's default source"; the engine fills it in on read. Storing
    /// the same id on every sense, example, form and pronunciation was 12.6% of the JSON.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleRecord {
    pub text: String,
    pub translation: Option<String>,
    /// Latin transliteration of `text`, for scripts the reader may not decode
    /// (Arabic, Chinese). Kaikki supplies this as `roman`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roman: Option<String>,
    /// Empty means "the pack's default source"; the engine fills it in on read. Storing
    /// the same id on every sense, example, form and pronunciation was 12.6% of the JSON.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRecord {
    pub target_lang: String,
    pub text: String,
    /// Empty means "the pack's default source"; the engine fills it in on read. Storing
    /// the same id on every sense, example, form and pronunciation was 12.6% of the JSON.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormRecord {
    pub form: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Empty means "the pack's default source"; the engine fills it in on read. Storing
    /// the same id on every sense, example, form and pronunciation was 12.6% of the JSON.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub entry_id: String,
    pub lemma: String,
    pub language: String,
    pub pos: Option<String>,
    pub matched_term: String,
    pub match_type: String, // "exact", "form", "loose"
}

impl EntryRecord {
    /// Visits every `source_id` in the record.
    fn for_each_source_id(&mut self, mut f: impl FnMut(&mut String)) {
        for p in &mut self.pronunciations {
            f(&mut p.source_id);
        }
        for form in &mut self.forms {
            f(&mut form.source_id);
        }
        for sense in &mut self.senses {
            f(&mut sense.source_id);
            for ex in &mut sense.examples {
                f(&mut ex.source_id);
            }
            for tr in &mut sense.translations {
                f(&mut tr.source_id);
            }
        }
    }

    /// Blanks every `source_id` equal to the pack default so serde omits it. Called by the
    /// pack builder before serializing; the pack records the default in its manifest table.
    pub fn strip_default_source(&mut self, default: &str) {
        self.for_each_source_id(|id| {
            if id == default {
                id.clear();
            }
        });
    }

    /// Restores every omitted `source_id` from the pack default. Called by the engine when
    /// an entry is read, so nothing downstream ever sees an empty provenance.
    pub fn fill_default_source(&mut self, default: &str) {
        self.for_each_source_id(|id| {
            if id.is_empty() {
                id.push_str(default);
            }
        });
    }
}
