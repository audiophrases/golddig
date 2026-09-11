// tts.ts — Neural Edge voice synthesis for Golddig desktop and web dictionary lookup.
// Prioritizes natural neural voices exposed by Microsoft Edge / WebView2 runtime.
// Reusable implementation based on proven speech pipeline.

let voices: SpeechSynthesisVoice[] = [];
const listeners = new Set<(v: SpeechSynthesisVoice[]) => void>();

let voicesSettled = false;
let settleTimer: any = null;
const readyWaiters = new Set<() => void>();

function haveNaturalVoice(): boolean {
  if (typeof window === 'undefined') return false;
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
  if (typeof window === 'undefined' || !window.speechSynthesis) {
    return;
  }
  if (voicesSettled) {
    cb();
  } else {
    readyWaiters.add(cb);
  }
}

function refresh() {
  if (typeof window === 'undefined' || !window.speechSynthesis) return;
  voices = window.speechSynthesis.getVoices();
  listeners.forEach((cb) => cb(voices));
  if (haveNaturalVoice()) {
    markVoicesSettled();
  }
}

if (typeof window !== 'undefined' && window.speechSynthesis) {
  refresh();
  window.speechSynthesis.onvoiceschanged = refresh;
  settleTimer = setTimeout(markVoicesSettled, 2000);
}

function rankVoice(v: SpeechSynthesisVoice): number {
  const n = v.name || '';
  let s = 0;
  if (/natural|neural/i.test(n)) s += 10; // High-quality Edge natural neural voices
  if (/online/i.test(n)) s += 5;
  if (/microsoft/i.test(n)) s += 3;
  if (/google/i.test(n)) s += 2;
  if (!v.localService) s += 1;
  return s;
}

// Language prefix mappings for 2-letter codes or specialized codes
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

export function getVoicesForLang(langCode: string): SpeechSynthesisVoice[] {
  const targets = LANG_MAP[langCode.toLowerCase()] || [langCode.toLowerCase()];
  const primaryPrefix = targets[0].slice(0, 2).toLowerCase();

  return voices
    .filter((v) => {
      const vLang = (v.lang || '').toLowerCase();
      return targets.some((t) => vLang === t.toLowerCase() || vLang.startsWith(t.toLowerCase())) ||
        vLang.startsWith(primaryPrefix);
    })
    .sort((a, b) => rankVoice(b) - rankVoice(a) || a.name.localeCompare(b.name));
}

export function getBestVoice(langCode: string): SpeechSynthesisVoice | null {
  const list = getVoicesForLang(langCode);
  return list[0] || null;
}

let keepAliveInterval: any = null;
let currentToken = 0;

export function speak(text: string, langCode: string = 'en', voiceName?: string | null, rate: number = 0.95) {
  if (typeof window === 'undefined' || !window.speechSynthesis || !text) return;

  stopSpeaking();
  const token = ++currentToken;

  whenVoicesReady(() => {
    if (token !== currentToken) return;

    const u = new SpeechSynthesisUtterance(text);
    const targetTags = LANG_MAP[langCode.toLowerCase()] || ['en-US'];
    u.lang = targetTags[0];

    let voice: SpeechSynthesisVoice | null = null;
    if (voiceName) {
      voice = voices.find((v) => v.name === voiceName) || null;
    }
    if (!voice) {
      voice = getBestVoice(langCode);
    }
    if (voice) {
      u.voice = voice;
      u.lang = voice.lang;
    }

    u.rate = Math.min(2, Math.max(0.5, rate));
    u.pitch = 1.0;

    u.onend = () => clearInterval(keepAliveInterval);
    u.onerror = () => clearInterval(keepAliveInterval);

    window.speechSynthesis.speak(u);

    // Keepalive for Chromium/Edge long-utterance bug
    clearInterval(keepAliveInterval);
    keepAliveInterval = setInterval(() => {
      if (window.speechSynthesis && window.speechSynthesis.speaking) {
        window.speechSynthesis.resume();
      } else {
        clearInterval(keepAliveInterval);
      }
    }, 5000);
  });
}

export function stopSpeaking() {
  currentToken++;
  if (keepAliveInterval) clearInterval(keepAliveInterval);
  if (typeof window !== 'undefined' && window.speechSynthesis) {
    window.speechSynthesis.cancel();
  }
}
