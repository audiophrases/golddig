// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Speech for Golddig headwords and examples.
//
// Two routes, tried in this order:
//
//   1. Microsoft neural voices, synthesised by the Rust side (src-tauri/src/tts.rs) through
//      the same Edge "Read Aloud" service the Edge browser uses. This is what makes the
//      speaker button work for Catalan and Moroccan Arabic, which have no local Windows
//      voice at all, and what makes English sound like Edge's "Natural" voice rather than
//      the SAPI one. It needs the network and it is user-initiated only — lookup itself
//      never leaves the machine — and it can fail (offline, or Microsoft changes the
//      endpoint), in which case the caller is told and route 2 is tried.
//
//   2. The browser Web Speech API (`window.speechSynthesis`) with whatever voices the host
//      exposes. Inside a Tauri WebView2 window that is the local SAPI set; Edge's own
//      online voices are injected by the Edge browser and are not available here. This
//      route never speaks without a voice that matches the language: an earlier version
//      fell through with no voice selected and read Catalan text aloud in English.
//
// `voiceStatusFor()` reports which route will be used so the UI can label the button
// honestly instead of pretending, and `speakNeural()` reports what actually happened.

import { invoke } from '@tauri-apps/api/core';

let voices: SpeechSynthesisVoice[] = [];
const listeners = new Set<(v: SpeechSynthesisVoice[]) => void>();

let voicesSettled = false;
let settleTimer: ReturnType<typeof setTimeout> | null = null;
const readyWaiters = new Set<() => void>();

export function speechAvailable(): boolean {
  return typeof window !== 'undefined' && !!window.speechSynthesis;
}

/**
 * Languages the Rust side maps to a neural voice — mirror of `voice_for` in
 * src-tauri/src/tts.rs, so the button can name the voice before the first click. The
 * name the service actually used comes back with every clip, so drift here would show.
 */
const NEURAL_VOICE: Record<string, string> = {
  en: 'en-US-JennyNeural',
  ca: 'ca-ES-JoanaNeural',
  es: 'es-ES-AlvaroNeural',
  fr: 'fr-FR-DeniseNeural',
  de: 'de-DE-KatjaNeural',
  ary: 'ar-MA-MounaNeural',
  ar: 'ar-MA-MounaNeural',
  zh: 'zh-CN-XiaoxiaoNeural',
  cmn: 'zh-CN-XiaoxiaoNeural',
};

/** Tauri 2 injects this into every webview it owns; a plain browser tab has no backend. */
function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** The neural voice that will be tried for `langCode`, or null when there is no backend. */
export function neuralVoiceFor(langCode: string): string | null {
  if (!inTauri()) return null;
  return NEURAL_VOICE[langCode.toLowerCase()] ?? null;
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
  | { kind: 'ready'; voiceName: string; cloud: boolean; neural: boolean }
  | { kind: 'unsupported'; message: string }
  | { kind: 'no-voice'; message: string };

/** Whether the local Web Speech route can speak this language, and with what. */
function localVoiceStatusFor(langCode: string): VoiceStatus {
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
  return { kind: 'ready', voiceName: voice.name, cloud: !voice.localService, neural: false };
}

/**
 * Whether this language can be spoken here, and by what. The UI uses this to label the
 * speaker button — and, when neither route has a voice, to disable it rather than play
 * the wrong language.
 */
export function voiceStatusFor(langCode: string): VoiceStatus {
  const neural = neuralVoiceFor(langCode);
  if (neural) return { kind: 'ready', voiceName: neural, cloud: true, neural: true };
  return localVoiceStatusFor(langCode);
}

/** Subscribe to voice-list changes so the UI can re-evaluate button state. */
export function onVoicesChanged(cb: (v: SpeechSynthesisVoice[]) => void): () => void {
  listeners.add(cb);
  cb(voices);
  return () => listeners.delete(cb);
}

let keepAliveInterval: ReturnType<typeof setInterval> | null = null;
let currentToken = 0;

/** The neural clip currently playing, so a new request or a stop can cut it off. */
let currentClip: { audio: HTMLAudioElement; release: () => void } | null = null;

/**
 * Speaks `text` in `langCode` with the local Web Speech voice. Returns the status it acted
 * on; callers can surface a `no-voice` result instead of silently producing the wrong
 * language.
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

  return localVoiceStatusFor(langCode);
}

/** Shape of the `speak_neural` command's reply (src-tauri/src/lib.rs, `SpeechClip`). */
interface SpeechClip {
  audio_base64: string;
  voice: string;
  cached: boolean;
}

function base64ToBlob(base64: string, type: string): Blob {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return new Blob([bytes], { type });
}

/** Plays an MP3 clip from a blob URL. Resolves once playback has started. */
function playClip(audioBase64: string): Promise<void> {
  const url = URL.createObjectURL(base64ToBlob(audioBase64, 'audio/mpeg'));
  const audio = new Audio(url);
  const release = () => {
    URL.revokeObjectURL(url);
    if (currentClip?.audio === audio) currentClip = null;
  };
  currentClip = { audio, release };
  audio.addEventListener('ended', release, { once: true });
  audio.addEventListener('error', release, { once: true });
  return audio.play().catch((e) => {
    release();
    throw e;
  });
}

export type SpeakOutcome =
  /** A Microsoft neural voice spoke it. `cached` means no network round trip this time. */
  | { kind: 'neural'; voice: string; cached: boolean }
  /** The neural route failed for `reason`; the local Web Speech voice spoke it instead. */
  | { kind: 'local'; voiceName: string; reason: string }
  /** Nothing could speak it. */
  | { kind: 'failed'; message: string }
  /** A newer request replaced this one before it played. */
  | { kind: 'superseded' };

/**
 * Speaks `text` in `langCode`: the Microsoft neural voice first, the local voice if that
 * fails. Returns what actually happened so the UI can say so — a fallback to the local
 * voice is audible, and a reader who hears the wrong quality deserves to know why.
 */
export async function speakNeural(text: string, langCode: string): Promise<SpeakOutcome> {
  text = text.trim();
  if (!text) return { kind: 'failed', message: 'nothing to say' };

  stopSpeaking();
  const token = ++currentToken;

  let reason: string;
  if (neuralVoiceFor(langCode)) {
    try {
      const clip = await invoke<SpeechClip>('speak_neural', { text, lang: langCode });
      if (token !== currentToken) return { kind: 'superseded' };
      await playClip(clip.audio_base64);
      return { kind: 'neural', voice: clip.voice, cached: clip.cached };
    } catch (e) {
      reason = e instanceof Error ? e.message : String(e);
    }
  } else {
    reason = inTauri()
      ? `no neural voice is mapped for ${langCode}`
      : 'neural voices need the desktop app';
  }
  if (token !== currentToken) return { kind: 'superseded' };

  const local = speak(text, langCode);
  if (local.kind === 'ready') return { kind: 'local', voiceName: local.voiceName, reason };
  const name = LANG_NAME[langCode.toLowerCase()] || langCode;
  return {
    kind: 'failed',
    message: `${reason}; and there is no local ${name} voice to fall back to`,
  };
}

export function stopSpeaking() {
  currentToken++;
  if (keepAliveInterval) {
    clearInterval(keepAliveInterval);
    keepAliveInterval = null;
  }
  if (currentClip) {
    currentClip.audio.pause();
    currentClip.release();
    currentClip = null;
  }
  if (speechAvailable()) window.speechSynthesis.cancel();
}
