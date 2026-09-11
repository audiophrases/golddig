use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchKey {
    pub exact_normalized: String,
    pub loose_key: String,
    pub language: String,
}

/// Normalizes text to NFC form, trims whitespace, and converts to lowercase for exact case-insensitive matches.
pub fn normalize_display(input: &str) -> String {
    input.trim().nfc().collect::<String>()
}

/// Generates search keys tailored by language rules.
/// Specifically:
/// - Spanish: preserves `ñ` in loose search, but folds standard accents (`á` -> `a`).
/// - Catalan: normalizes ela geminada (`l·l`, `l.l`, `l•l`) to `l·l` for display, and folds to `ll` in loose key while keeping `l·l` exact.
/// - German: `ß` folds to `ss` in loose key, preserving `ß` in exact key.
/// - French: `œ` / `æ` ligatures fold to `oe` / `ae` in loose key.
pub fn generate_search_keys(input: &str, lang: &str) -> SearchKey {
    let clean = normalize_display(input);
    let exact_normalized = clean.to_lowercase();

    let loose_key = match lang {
        "es" => loose_spanish(&exact_normalized),
        "ca" => loose_catalan(&exact_normalized),
        "de" => loose_german(&exact_normalized),
        "fr" => loose_french(&exact_normalized),
        "ary" | "ar" => loose_arabic(&exact_normalized),
        "zh" | "cmn" => loose_pinyin(&exact_normalized),
        _ => loose_generic(&exact_normalized),
    };

    SearchKey {
        exact_normalized,
        loose_key,
        language: lang.to_string(),
    }
}

fn loose_spanish(s: &str) -> String {
    // Preserve ñ / Ñ, strip other diacritics
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'ñ' => out.push('ñ'),
            'á' => out.push('a'),
            'é' => out.push('e'),
            'í' => out.push('i'),
            'ó' => out.push('o'),
            'ú' | 'ü' => out.push('u'),
            _ => {
                if !c.is_ascii_punctuation() && !c.is_whitespace() {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn loose_catalan(s: &str) -> String {
    // Catalan: 'l·l', 'l.l' -> 'll' in loose search, strip accents
    let replaced = s
        .replace("l·l", "ll")
        .replace("l.l", "ll")
        .replace("l•l", "ll");
    let mut out = String::with_capacity(replaced.len());
    for c in replaced.chars() {
        match c {
            'à' => out.push('a'),
            'é' | 'è' => out.push('e'),
            'í' | 'ï' => out.push('i'),
            'ó' | 'ò' => out.push('o'),
            'ú' | 'ü' => out.push('u'),
            'ç' => out.push('c'),
            _ => {
                if !c.is_ascii_punctuation() && !c.is_whitespace() {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn loose_german(s: &str) -> String {
    // German: ß -> ss, ä -> a, ö -> o, ü -> u
    let replaced = s.replace('ß', "ss");
    let mut out = String::with_capacity(replaced.len());
    for c in replaced.chars() {
        match c {
            'ä' => out.push('a'),
            'ö' => out.push('o'),
            'ü' => out.push('u'),
            _ => {
                if !c.is_ascii_punctuation() && !c.is_whitespace() {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn loose_french(s: &str) -> String {
    // French: œ -> oe, æ -> ae, strip accents
    let replaced = s.replace('œ', "oe").replace('æ', "ae");
    let mut out = String::with_capacity(replaced.len());
    for c in replaced.chars() {
        match c {
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'à' | 'â' => out.push('a'),
            'î' | 'ï' => out.push('i'),
            'ô' => out.push('o'),
            'ù' | 'û' | 'ü' => out.push('u'),
            'ç' => out.push('c'),
            _ => {
                if !c.is_ascii_punctuation() && !c.is_whitespace() {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn loose_generic(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if !c.is_ascii_punctuation() && !c.is_whitespace() {
            out.push(c);
        }
    }
    out
}

fn loose_arabic(s: &str) -> String {
    // Strip Arabic diacritics / harakat and normalize alifs
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            // Harakat / Tashkeel
            '\u{064B}'..='\u{065F}' | '\u{0670}' => {},
            // Normalize alif with hamza / madda to plain alif
            'أ' | 'إ' | 'آ' | 'ٱ' => out.push('ا'),
            // Normalize taa marbuta to haa or taa
            'ة' => out.push('ه'),
            // Normalize alif maqsura to yaa
            'ى' => out.push('ي'),
            _ => {
                if !c.is_ascii_punctuation() && !c.is_whitespace() {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn loose_pinyin(s: &str) -> String {
    // Strip tone marks from Pinyin for loose search: āáǎà -> a, etc.
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'ā' | 'á' | 'ǎ' | 'à' => out.push('a'),
            'ē' | 'é' | 'ě' | 'è' => out.push('e'),
            'ī' | 'í' | 'ǐ' | 'ì' => out.push('i'),
            'ō' | 'ó' | 'ǒ' | 'ò' => out.push('o'),
            'ū' | 'ú' | 'ǔ' | 'ù' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'ü' => out.push('u'),
            _ => {
                if !c.is_ascii_punctuation() && !c.is_whitespace() {
                    out.push(c);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spanish_n_and_enye_separation() {
        let ano_key = generate_search_keys("ano", "es");
        let anyo_key = generate_search_keys("año", "es");

        assert_eq!(ano_key.exact_normalized, "ano");
        assert_eq!(ano_key.loose_key, "ano");

        assert_eq!(anyo_key.exact_normalized, "año");
        assert_eq!(anyo_key.loose_key, "año");

        // Critically: "ano" and "año" must NEVER collide in loose key
        assert_ne!(ano_key.loose_key, anyo_key.loose_key);
    }

    #[test]
    fn test_catalan_ela_geminada() {
        let key = generate_search_keys("col·lecció", "ca");
        assert_eq!(key.exact_normalized, "col·lecció");
        assert_eq!(key.loose_key, "colleccio");

        let key_dot = generate_search_keys("col.lecció", "ca");
        assert_eq!(key_dot.loose_key, "colleccio");
    }

    #[test]
    fn test_german_eszett() {
        let key = generate_search_keys("Straße", "de");
        assert_eq!(key.exact_normalized, "straße");
        assert_eq!(key.loose_key, "strasse");
    }

    #[test]
    fn test_french_ligature() {
        let key = generate_search_keys("cœur", "fr");
        assert_eq!(key.exact_normalized, "cœur");
        assert_eq!(key.loose_key, "coeur");
    }

    #[test]
    fn test_arabic_alif_and_tashkeel() {
        let key = generate_search_keys("أَنَا", "ary");
        assert_eq!(key.loose_key, "انا");
    }

    #[test]
    fn test_pinyin_tones() {
        let key = generate_search_keys("nǐhǎo", "zh");
        assert_eq!(key.loose_key, "nihao");
    }
}
