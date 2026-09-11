// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Speech synthesis for Golddig headwords and examples.
//
// This is the browser Web Speech API (`window.speechSynthesis`) using whatever voices
// the host platform exposes. It is NOT Microsoft Edge neural TTS, and an earlier version
// of this file claimed to be: it only ranked voices by matching /natural|neural|online/
// against their names. Two things follow from that, and the UI depends on both:
//
//   1. Edge's "… Online (Natural)" voices are injected by the Edge browser itself. A
//      Tauri WebView2 window does not get them, so inside golddig.exe you hear the local
//      SAPI voice. They are also cloud-synthesised, so they would need network — which
//      the rest of the app deliberately never uses.
//   2. Windows ships no Catalan and no Moroccan Arabic voice by default. Previously
//      `speak()` would fall through with no voice selected and the platform would read
//      Catalan text aloud in English. `voiceStatusFor()` now reports that instead, and
//      the UI disables the button.
//
// For real offline neural audio the route is a bundled Piper (ONNX) voice per language
// invoked from Rust, or shipping the Wiktionary recordings the importer now preserves in
// PronunciationRecord.audio. Both are tracked in docs/roadmap.md, neither is this file.

let voices: SpeechSynthesisVoice[] = [];
const listeners = new Set<(v: SpeechSynthesisVoice[]) => void>();

let voicesSettled = false;
let settleTimer: ReturnType<typeof setTimeout> | null = null;
const readyWaiters = new Set<() => void>();

export function speechAvailable(): boolean {
  return typeof window !== 'undefined' && !!window.speechSynthesis;
}

function haveHighQualityVoice(): boolean {
  return voices.some((v) => /natural|online|neural/i.test(v.name || ''));
}

function markVoicesSettled() {
  if (voicesSettled) return;
  voicesSettled = true;
  if (settleTimer) clearTimeout(settleTimer);
  readyWaiters.forEach((cb) => cb());
  readyWaiters.clear();
}

function whenVoicesReady(cb: () => void) {
  if (!speechAvailable()) return;
  if (voicesSettled) cb();
  else readyWaiters.add(cb);
}

function refresh() {
  if (!speechAvailable()) return;
  voices = window.speechSynthesis.getVoices();
  listeners.forEach((cb) => cb(voices));
  // Edge publishes its online voices a moment after load; settle early if they arrive.
  if (haveHighQualityVoice()) markVoicesSettled();
}

if (speechAvailable()) {
  refresh();
  window.speechSynthesis.onvoiceschanged = refresh;
  settleTimer = setTimeout(markVoicesSettled, 2000);
}

/** Prefer cloud "natural" voices where the host actually exposes them. */
function rankVoice(v: SpeechSynthesisVoice): number {
  const n = v.name || '';
  let score = 0;
  if (/natural|neural/i.test(n)) score += 10;
  if (/online/i.test(n)) score += 5;
  if (/microsoft/i.test(n)) score += 3;
  if (/google/i.test(n)) score += 2;
  if (!v.localService) score += 1;
  return score;
}

const LANG_MAP: Record<string, string[]> = {
  en: ['en-US', 'en-GB', 'en'],
  ca: ['ca-ES', 'ca'],
  es: ['es-ES', 'es-MX', 'es'],
  fr: ['fr-FR', 'fr-CA', 'fr'],
  de: ['de-DE', 'de-AT', 'de'],
  ary: ['ar-MA', 'ar-SA', 'ar-EG', 'ar'],
  ar: ['ar-MA', 'ar-SA', 'ar-EG', 'ar'],
  zh: ['zh-CN', 'zh-TW', 'zh-HK', 'zh'],
  cmn: ['zh-CN', 'zh-TW', 'zh'],
};

/** Human-readable language name for status messages. */
const LANG_NAME: Record<string, string> = {
  en: 'English',
  ca: 'Catalan',
  es: 'Spanish',
  fr: 'French',
  de: 'German',
  ary: 'Moroccan Arabic',
  ar: 'Arabic',
  zh: 'Chinese',
  cmn: 'Chinese',
};

export function getVoicesForLang(langCode: string): SpeechSynthesisVoice[] {
  const code = langCode.toLowerCase();
  const targets = LANG_MAP[code] || [code];
  // Match only the requested language. The previous implementation also accepted any
  // voice whose tag merely began with the same two letters as the primary target, which
  // combined with a null result let the platform substitute its default English voice.
  return voices
    .filter((v) => {
      const vLang = (v.lang || '').toLowerCase().replace('_', '-');
      return targets.some((t) => {
        const tl = t.toLowerCase();
        return vLang === tl || vLang.startsWith(`${tl}-`);
      });
    })
    .sort((a, b) => rankVoice(b) - rankVoice(a) || a.name.localeCompare(b.name));
}

export function getBestVoice(langCode: string): SpeechSynthesisVoice | null {
  return getVoicesForLang(langCode)[0] || null;
}

export type VoiceStatus =
  | { kind: 'ready'; voiceName: string; cloud: boolean }
  | { kind: 'unsupported'; message: string }
  | { kind: 'no-voice'; message: string };

/**
 * Whether this language can actually be spoken here, and by what. The UI uses this to
 * disable the speaker button rather than play the wrong language.
 */
export function voiceStatusFor(langCode: string): VoiceStatus {
  if (!speechAvailable()) {
    return { kind: 'unsupported', message: 'Speech synthesis is unavailable in this window' };
  }
  const voice = getBestVoice(langCode);
  if (!voice) {
    const name = LANG_NAME[langCode.toLowerCase()] || langCode;
    return {
      kind: 'no-voice',
      message: `No ${name} speech voice is installed on this system`,
    };
  }
  return { kind: 'ready', voiceName: voice.name, cloud: !voice.localService };
}

/** Subscribe to voice-list changes so the UI can re-evaluate button state. */
export function onVoicesChanged(cb: (v: SpeechSynthesisVoice[]) => void): () => void {
  listeners.add(cb);
  cb(voices);
  return () => listeners.delete(cb);
}

let keepAliveInterval: ReturnType<typeof setInterval> | null = null;
let currentToken = 0;

/**
 * Speaks `text` in `langCode`. Returns the status it acted on; callers can surface a
 * `no-voice` result instead of silently producing the wrong language.
 */
export function speak(
  text: string,
  langCode: string = 'en',
  voiceName?: string | null,
  rate: number = 0.95
): VoiceStatus {
  if (!speechAvailable() || !text) {
    return { kind: 'unsupported', message: 'Speech synthesis is unavailable in this window' };
  }

  stopSpeaking();
  const token = ++currentToken;

  whenVoicesReady(() => {
    if (token !== currentToken) return;

    const explicit = voiceName ? voices.find((v) => v.name === voiceName) : null;
    const voice = explicit || getBestVoice(langCode);
    // Never speak without a matching voice — that is what read Catalan in English.
    if (!voice) return;

    const utterance = new SpeechSynthesisUtterance(text);
    utterance.voice = voice;
    utterance.lang = voice.lang;
    utterance.rate = Math.min(2, Math.max(0.5, rate));
    utterance.pitch = 1.0;

    const clearKeepAlive = () => {
      if (keepAliveInterval) clearInterval(keepAliveInterval);
      keepAliveInterval = null;
    };
    utterance.onend = clearKeepAlive;
    utterance.onerror = clearKeepAlive;

    window.speechSynthesis.speak(utterance);

    // Works around the Chromium/Edge bug that halts long utterances after ~15s.
    clearKeepAlive();
    keepAliveInterval = setInterval(() => {
      if (window.speechSynthesis?.speaking) window.speechSynthesis.resume();
      else clearKeepAlive();
    }, 5000);
  });

  return voiceStatusFor(langCode);
}

export function stopSpeaking() {
  currentToken++;
  if (keepAliveInterval) {
    clearInterval(keepAliveInterval);
    keepAliveInterval = null;
  }
  if (speechAvailable()) window.speechSynthesis.cancel();
}
