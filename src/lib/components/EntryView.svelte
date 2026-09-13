<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/. -->

<script lang="ts">
  import type { EntryRecord, FormRecord } from '../types';
  import { speakNeural, voiceStatusFor, onVoicesChanged } from '../tts';

  let { entry }: { entry: EntryRecord } = $props();

  // Re-evaluated when the platform publishes its voice list (Edge does so late).
  let voiceTick = $state(0);
  onVoicesChanged(() => voiceTick++);

  const voice = $derived.by(() => {
    void voiceTick;
    return voiceStatusFor(entry.language);
  });
  const canSpeak = $derived(voice.kind === 'ready');
  const speakTitle = $derived(
    voice.kind !== 'ready'
      ? voice.message
      : voice.neural
        ? `Speak with the Microsoft neural voice ${voice.voiceName} — needs network`
        : `Speak with ${voice.voiceName}${voice.cloud ? ' (cloud voice, needs network)' : ''}`
  );

  // A neural clip is a network round trip (about a second). The button shows it is busy
  // rather than looking dead, and anything short of the neural voice is reported here
  // instead of letting the reader wonder why the audio sounds different.
  let speaking = $state(false);
  let speechNote = $state('');

  async function say(text: string) {
    speaking = true;
    speechNote = '';
    try {
      const outcome = await speakNeural(text, entry.language);
      if (outcome.kind === 'local') {
        speechNote = `Neural voice unavailable (${outcome.reason}) — using the local voice ${outcome.voiceName}`;
      } else if (outcome.kind === 'failed') {
        speechNote = `Could not speak: ${outcome.message}`;
      }
    } finally {
      speaking = false;
    }
  }

  // A note about one entry's audio has no business lingering under the next.
  $effect(() => {
    void entry.id;
    speechNote = '';
  });

  /**
   * Inflections worth showing. A "FORMS" row was previously rendered for 98.8% of
   * English entries, and for 6.8% of them it only repeated the headword: the importer
   * labels a form with no tags as the generic kind "form", and emits the canonical
   * spelling as a form of itself.
   */
  const displayForms = $derived.by(() => {
    const seen = new Set<string>();
    return (entry.forms ?? []).filter((f: FormRecord) => {
      const kind = (f.type || '').toLowerCase();
      if (!f.form || f.form === entry.lemma) return false;
      if (!kind || kind === 'form' || kind === 'canonical') return false;
      if (seen.has(f.form)) return false;
      seen.add(f.form);
      return true;
    });
  });

  const spoken = $derived((entry.pronunciations ?? []).filter((p) => p.ipa));
  const recordings = $derived((entry.pronunciations ?? []).filter((p) => p.audio));

  function playRecording(url: string) {
    // User-initiated only. Lookup itself never touches the network.
    void new Audio(url).play().catch(() => {});
  }
</script>

<article class="entry-view">
  <header class="entry-header">
    <div class="header-line">
      <h1 class="entry-lemma">{entry.lemma}</h1>
      <button
        class="tts-btn"
        class:busy={speaking}
        onclick={() => say(entry.lemma)}
        disabled={!canSpeak}
        aria-busy={speaking}
        title={speakTitle}
        aria-label={speakTitle}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
          <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
        </svg>
      </button>
      {#each recordings as rec (rec.audio)}
        <button
          class="tts-btn recording"
          onclick={() => playRecording(rec.audio!)}
          title="Play the Wiktionary recording{rec.tags?.length ? ` (${rec.tags.join(', ')})` : ''} — downloads on click"
          aria-label="Play recorded pronunciation"
        >
          rec
        </button>
      {/each}
      <span class="entry-lang">{entry.language}</span>
      {#if entry.pos}
        <span class="entry-pos">{entry.pos}</span>
      {/if}
    </div>

    {#if speechNote}
      <p class="speech-note" role="status">{speechNote}</p>
    {/if}

    {#if spoken.length > 0}
      <div class="pronunciations">
        {#each spoken as pron (pron.ipa + (pron.tags ?? []).join())}
          <span class="ipa">
            {pron.ipa}
            {#if pron.tags && pron.tags.length > 0}
              <span class="ipa-tag">{pron.tags.join(', ')}</span>
            {/if}
          </span>
        {/each}
      </div>
    {/if}

    {#if displayForms.length > 0}
      <details class="forms">
        <summary>Inflections ({displayForms.length})</summary>
        <div class="forms-body">
          {#each displayForms as form (form.form + form.type)}
            <span class="form-badge">{form.form}<span class="form-type">{form.type}</span></span>
          {/each}
        </div>
      </details>
    {/if}
  </header>

  <section class="senses-section">
    <ol class="senses-list">
      {#each entry.senses as sense, idx (sense.id)}
        <li class="sense-item">
          <div class="sense-def">
            <span class="sense-num">{idx + 1}.</span>
            <span class="def-text">
              {#if sense.tags && sense.tags.length > 0}
                <span class="sense-tags">{sense.tags.join(', ')}</span>
              {/if}
              {sense.definition}
            </span>
          </div>

          {#if sense.translations && sense.translations.length > 0}
            <div class="block">
              <span class="sub-label">Translations</span>
              <ul class="inline-list">
                {#each sense.translations as tr, i (tr.target_lang + tr.text + i)}
                  <li><span class="tr-lang">{tr.target_lang}</span> <strong>{tr.text}</strong></li>
                {/each}
              </ul>
            </div>
          {/if}

          {#if sense.examples && sense.examples.length > 0}
            <div class="block">
              <span class="sub-label">Examples</span>
              <ul class="examples-list">
                {#each sense.examples as ex, i (ex.text + i)}
                  <li class="example-item">
                    <span class="ex-quote">{ex.text}</span>
                    <button
                      class="tts-mini-btn"
                      class:busy={speaking}
                      onclick={() => say(ex.text)}
                      disabled={!canSpeak}
                      aria-busy={speaking}
                      title={speakTitle}
                      aria-label="Speak this example"
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
                      </svg>
                    </button>
                    {#if ex.roman}
                      <span class="ex-roman">{ex.roman}</span>
                    {/if}
                    {#if ex.translation}
                      <span class="ex-trans">{ex.translation}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            </div>
          {/if}

          {#if sense.synonyms && sense.synonyms.length > 0}
            <div class="block inline">
              <span class="sub-label">Synonyms</span>
              <span class="rel-text">{sense.synonyms.join(' · ')}</span>
            </div>
          {/if}

          {#if sense.collocations && sense.collocations.length > 0}
            <div class="block inline">
              <span class="sub-label">Commonly used with</span>
              <span class="rel-text">{sense.collocations.join(' · ')}</span>
            </div>
          {/if}

          <footer class="sense-source">{sense.source_id}</footer>
        </li>
      {/each}
    </ol>
  </section>
</article>

<style>
  .entry-view {
    padding: 0.25rem 0 2rem;
  }
  .entry-header {
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.75rem;
    margin-bottom: 1rem;
  }
  .header-line {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  /* Explicit, so the headword does not inherit a stray global serif rule. */
  .entry-lemma {
    margin: 0;
    font-family: var(--font-serif);
    font-size: 1.8rem;
    font-weight: 600;
    line-height: 1.15;
    color: var(--text-h);
    overflow-wrap: anywhere;
  }
  .tts-btn {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.2rem 0.4rem;
    color: var(--accent);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    line-height: 1;
  }
  .tts-btn.recording {
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .tts-btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-bg);
  }
  .tts-btn:disabled,
  .tts-mini-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  /* Fetching a neural clip: visibly working, still clickable so it can never wedge. */
  .tts-btn.busy,
  .tts-mini-btn.busy {
    animation: tts-pulse 0.9s ease-in-out infinite;
  }
  @keyframes tts-pulse {
    50% {
      opacity: 0.4;
    }
  }
  .speech-note {
    margin: 0.3rem 0 0;
    font-size: 0.78rem;
    color: var(--text-muted);
  }
  .entry-lang {
    font-size: 0.8rem;
    color: var(--accent);
    font-weight: 600;
    text-transform: uppercase;
  }
  .entry-pos {
    font-size: 0.9rem;
    color: var(--text-muted);
    font-style: italic;
  }
  .pronunciations {
    margin-top: 0.35rem;
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .ipa-tag {
    font-family: var(--font-ui);
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .forms {
    margin-top: 0.5rem;
    font-size: 0.8rem;
  }
  .forms summary {
    cursor: pointer;
    color: var(--text-muted);
  }
  .forms-body {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.4rem;
  }
  .form-badge {
    display: inline-flex;
    gap: 0.3rem;
    align-items: baseline;
    background: var(--code-bg);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
  }
  .form-type {
    color: var(--text-muted);
    font-size: 0.7rem;
  }
  .senses-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }
  .sense-def {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
  }
  .sense-num {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    flex: none;
  }
  .def-text {
    color: var(--text-h);
  }
  .sense-tags {
    font-size: 0.75rem;
    font-style: italic;
    color: var(--accent);
    margin-right: 0.3rem;
  }
  .block {
    margin: 0.45rem 0 0 1.35rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .block.inline {
    flex-direction: row;
    gap: 0.5rem;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .sub-label {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    font-weight: 600;
    color: var(--text-muted);
    flex: none;
  }
  .inline-list,
  .examples-list {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 0.92rem;
  }
  .inline-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.15rem 0.9rem;
  }
  .tr-lang {
    color: var(--text-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
  }
  .examples-list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .example-item {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    align-items: baseline;
  }
  .ex-quote {
    font-style: italic;
  }
  .ex-roman {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .ex-trans {
    color: var(--text-muted);
  }
  .ex-trans::before {
    content: '— ';
  }
  .tts-mini-btn {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    line-height: 1;
  }
  .rel-text {
    font-size: 0.92rem;
  }
  .sense-source {
    margin: 0.45rem 0 0 1.35rem;
    font-size: 0.68rem;
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
</style>
