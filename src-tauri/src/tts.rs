// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Neural speech synthesis through the Microsoft Edge "Read Aloud" service.
//!
//! Why this exists. The speaker button used to call the browser's `speechSynthesis`, which
//! inside a Tauri WebView2 window exposes only the locally installed SAPI voices. Windows
//! ships none for Catalan or Moroccan Arabic, so for those languages the button was simply
//! disabled — and even for English the voice was the robotic local one, not the "Natural"
//! voices Edge advertises. Those are not a WebView2 feature; Edge injects them from a cloud
//! service. This module talks to that same service directly, the way Edge's own Read Aloud
//! does, and gets the same voices — including `ca-ES-JoanaNeural` and `ar-MA-MounaNeural`.
//!
//! What it is not. This is not a documented public API. It is the endpoint Edge itself uses,
//! reverse-engineered and kept working by the open-source `edge-tts` project, and Microsoft
//! could change it. Every caller therefore treats failure as normal: the frontend falls back
//! to the local `speechSynthesis` voice, and the button explains what happened. It also needs
//! the network. Dictionary lookup never does; audio is a separate, user-initiated action,
//! exactly like the Wiktionary recording button that fetches from Wikimedia on click.
//!
//! Synthesized clips are cached in memory per (voice, text), bounded, so replaying a headword
//! does not round-trip again.

use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

/// The token Edge's Read Aloud embeds. Public, and identical for every Edge install.
const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";

/// The Edge build the service is told it is talking to. The service rejects the WebSocket
/// upgrade with 403 once this falls too far behind current Edge — it did with 130.x while
/// the voice-list endpoint still accepted it, which is why the two can look inconsistent.
/// When synthesis starts failing with 403 this is the first thing to bump; the current
/// value is `CHROMIUM_FULL_VERSION` in
/// <https://github.com/rany2/edge-tts/blob/master/src/edge_tts/constants.py>.
const CHROMIUM_FULL_VERSION: &str = "143.0.3650.75";
const CHROMIUM_MAJOR_VERSION: &str = "143";
const ENDPOINT: &str =
    "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";

/// Audio format requested from the service. MP3 at 24 kHz is what Edge itself asks for and
/// what every browser can play from a blob URL without extra codecs.
const OUTPUT_FORMAT: &str = "audio-24khz-48kbitrate-mono-mp3";

/// Bounded cache of synthesized clips. Small on purpose: clips are ~50 KB and a reader
/// rarely replays more than a handful.
const CACHE_LIMIT: usize = 64;

/// Which neural voice to use for each Golddig language code.
///
/// Every language Golddig ships a pack for has a voice here — that was the whole point.
pub fn voice_for(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "en" => "en-US-JennyNeural",
        "ca" => "ca-ES-JoanaNeural",
        "es" => "es-ES-AlvaroNeural",
        "fr" => "fr-FR-DeniseNeural",
        "de" => "de-DE-KatjaNeural",
        "ary" | "ar" => "ar-MA-MounaNeural",
        "zh" | "cmn" => "zh-CN-XiaoxiaoNeural",
        _ => return None,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("no neural voice is mapped for language {0:?}")]
    NoVoice(String),
    #[error("text is empty")]
    EmptyText,
    #[error("could not reach the Edge speech service: {0}")]
    Connect(String),
    #[error("the Edge speech service returned no audio")]
    NoAudio,
    #[error("Edge speech service protocol error: {0}")]
    Protocol(String),
}

/// The `Sec-MS-GEC` header the service requires since late 2024: a SHA-256 over the current
/// time (as Windows file-time ticks, rounded down to a five-minute boundary) and the
/// trusted client token. Derived from the `edge-tts` implementation.
fn sec_ms_gec() -> String {
    const WINDOWS_EPOCH_OFFSET_SECS: u64 = 11_644_473_600;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut ticks = now + WINDOWS_EPOCH_OFFSET_SECS;
    ticks -= ticks % 300;
    let ticks = ticks as u128 * 10_000_000;
    let digest = Sha256::digest(format!("{ticks}{TRUSTED_CLIENT_TOKEN}").as_bytes());
    digest.iter().map(|b| format!("{b:02X}")).collect()
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// The service timestamps its messages in this exact shape.
fn timestamp() -> String {
    // e.g. "Sat Sep 13 2026 18:40:20 GMT+0000 (Coordinated Universal Time)"
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil date from days since epoch (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    let wd = (days + 4).rem_euclid(7);
    const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MO: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        WD[wd as usize],
        MO[(mo - 1) as usize],
        d,
        y,
        h,
        m,
        s
    )
}

/// Synthesizes `text` with `voice` and returns MP3 bytes.
///
/// One WebSocket session per call: send the speech configuration, send the SSML, collect
/// the binary audio frames until `turn.end`. Bounded by a timeout so a stalled service
/// can never hang the UI.
pub async fn synthesize(text: &str, voice: &str) -> Result<Vec<u8>, TtsError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(TtsError::EmptyText);
    }

    let connection_id = uuid::Uuid::new_v4().simple().to_string();
    let url = format!(
        "{ENDPOINT}?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&Sec-MS-GEC={}&Sec-MS-GEC-Version=1-{CHROMIUM_FULL_VERSION}&ConnectionId={connection_id}",
        sec_ms_gec()
    );

    let mut request = url
        .into_client_request()
        .map_err(|e| TtsError::Connect(e.to_string()))?;
    {
        let headers = request.headers_mut();
        let hv = |v: &str| v.parse().expect("static header value");
        headers.insert("Pragma", hv("no-cache"));
        headers.insert("Cache-Control", hv("no-cache"));
        headers.insert(
            "Origin",
            hv("chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold"),
        );
        headers.insert("Accept-Encoding", hv("gzip, deflate, br, zstd"));
        headers.insert("Accept-Language", hv("en-US,en;q=0.9"));
        // Edge reports only its major version in the User-Agent.
        headers.insert(
            "User-Agent",
            hv(&format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/{CHROMIUM_MAJOR_VERSION}.0.0.0 Safari/537.36 Edg/{CHROMIUM_MAJOR_VERSION}.0.0.0"
            )),
        );
    }

    let connect = tokio_tungstenite::connect_async(request);
    let (mut socket, _) = tokio::time::timeout(std::time::Duration::from_secs(10), connect)
        .await
        .map_err(|_| TtsError::Connect("timed out".into()))?
        .map_err(|e| TtsError::Connect(e.to_string()))?;

    let stamp = timestamp();
    let config = format!(
        "X-Timestamp:{stamp}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n\
         {{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"{OUTPUT_FORMAT}\"}}}}}}}}\r\n"
    );
    socket
        .send(Message::Text(config))
        .await
        .map_err(|e| TtsError::Protocol(e.to_string()))?;

    let request_id = uuid::Uuid::new_v4().simple().to_string();
    let ssml = format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
         <voice name='{voice}'><prosody pitch='+0Hz' rate='+0%' volume='+0%'>{}</prosody></voice></speak>",
        xml_escape(text)
    );
    let ssml_message = format!(
        "X-RequestId:{request_id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{stamp}Z\r\nPath:ssml\r\n\r\n{ssml}"
    );
    socket
        .send(Message::Text(ssml_message))
        .await
        .map_err(|e| TtsError::Protocol(e.to_string()))?;

    let mut audio = Vec::new();
    let deadline = std::time::Duration::from_secs(20);
    let collect = async {
        while let Some(frame) = socket.next().await {
            match frame.map_err(|e| TtsError::Protocol(e.to_string()))? {
                Message::Text(text) => {
                    if text.contains("Path:turn.end") {
                        break;
                    }
                }
                Message::Binary(bytes) => {
                    // Frame layout: 2-byte big-endian header length, the headers, then audio.
                    if bytes.len() < 2 {
                        continue;
                    }
                    let header_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
                    if bytes.len() < 2 + header_len {
                        continue;
                    }
                    let headers = String::from_utf8_lossy(&bytes[2..2 + header_len]);
                    if headers.contains("Path:audio") {
                        audio.extend_from_slice(&bytes[2 + header_len..]);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        Ok::<(), TtsError>(())
    };
    tokio::time::timeout(deadline, collect)
        .await
        .map_err(|_| TtsError::Connect("timed out waiting for audio".into()))??;

    let _ = socket.close(None).await;

    if audio.is_empty() {
        return Err(TtsError::NoAudio);
    }
    Ok(audio)
}

/// Bounded in-memory cache of synthesized clips, keyed by (voice, text).
#[derive(Default)]
pub struct ClipCache {
    clips: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl ClipCache {
    pub fn get(&self, voice: &str, text: &str) -> Option<Vec<u8>> {
        self.clips
            .lock()
            .ok()?
            .get(&(voice.to_string(), text.to_string()))
            .cloned()
    }

    pub fn put(&self, voice: &str, text: &str, clip: Vec<u8>) {
        if let Ok(mut map) = self.clips.lock() {
            if map.len() >= CACHE_LIMIT {
                map.clear();
            }
            map.insert((voice.to_string(), text.to_string()), clip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_shipped_language_has_a_voice() {
        for lang in ["en", "ca", "es", "fr", "de", "ary", "zh"] {
            assert!(
                voice_for(lang).is_some(),
                "no neural voice mapped for {lang}"
            );
        }
        assert!(voice_for("xx").is_none());
    }

    #[test]
    fn test_token_is_uppercase_sha256_hex() {
        let token = sec_ms_gec();
        assert_eq!(token.len(), 64);
        assert!(token
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
    }

    #[test]
    fn test_ssml_escaping_keeps_markup_literal() {
        assert_eq!(xml_escape("a < b & c"), "a &lt; b &amp; c");
    }

    #[test]
    fn test_timestamp_has_the_service_shape() {
        let t = timestamp();
        assert!(t.ends_with("GMT+0000 (Coordinated Universal Time)"), "{t}");
        assert_eq!(&t[3..4], " ");
    }

    /// Exercises the real service. Needs the network and an endpoint Microsoft could change,
    /// so it is opt-in: `cargo test -- --ignored test_synthesize_live`.
    #[test]
    #[ignore]
    fn test_synthesize_live() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mp3 = rt
            .block_on(synthesize("hola", voice_for("ca").unwrap()))
            .expect("live synthesis");
        // MP3 frames begin with an 0xFF sync byte or an ID3 tag.
        assert!(mp3.len() > 1000, "got {} bytes", mp3.len());
        assert!(
            mp3.starts_with(b"ID3") || mp3[0] == 0xFF,
            "not MP3: {:?}",
            &mp3[..4]
        );
    }
}
