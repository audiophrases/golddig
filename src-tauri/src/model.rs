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
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRecord {
    pub target_lang: String,
    pub text: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormRecord {
    pub form: String,
    #[serde(rename = "type")]
    pub kind: String,
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
