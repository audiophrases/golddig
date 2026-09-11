<script lang="ts">
  import type { EntryRecord } from '../types';
  import { speak } from '../tts';

  let { entry }: { entry: EntryRecord } = $props();

  function playAudio(text: string, lang: string) {
    speak(text, lang);
  }
</script>

<article class="entry-view">
  <header class="entry-header">
    <div class="header-line">
      <h1 class="entry-lemma">{entry.lemma}</h1>
      <button
        class="tts-btn"
        onclick={() => playAudio(entry.lemma, entry.language)}
        title="Listen with Neural Edge Voice ({entry.language})"
        aria-label="Speak pronunciation"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
          <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
          <path d="M19.07 4.93a10 10 0 0 1 0 14.14"></path>
        </svg>
      </button>
      <span class="entry-lang">{entry.language}</span>
      {#if entry.pos}
        <span class="entry-pos">({entry.pos})</span>
      {/if}
    </div>

    {#if entry.pronunciations && entry.pronunciations.length > 0}
      <div class="pronunciations">
        {#each entry.pronunciations as pron}
          <span class="ipa" title="IPA pronunciation from {pron.source_id}">
            {pron.ipa}
          </span>
        {/each}
      </div>
    {/if}

    {#if entry.forms && entry.forms.length > 0}
      <div class="forms">
        <span class="forms-label">Forms:</span>
        {#each entry.forms as form}
          <span class="form-badge">{form.form} <span class="form-type">[{form.type}]</span></span>
        {/each}
      </div>
    {/if}
  </header>

  <section class="senses-section">
    <ol class="senses-list">
      {#each entry.senses as sense, idx}
        <li class="sense-item">
          <div class="sense-def">
            <span class="sense-num">{idx + 1}.</span>
            <span class="def-text">{sense.definition}</span>
          </div>

          {#if sense.translations && sense.translations.length > 0}
            <div class="translations-block">
              <span class="sub-label">Translations:</span>
              <ul class="inline-list">
                {#each sense.translations as tr}
                  <li class="tr-item">
                    <span class="tr-lang">{tr.target_lang}:</span>
                    <strong class="tr-text">{tr.text}</strong>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}

          {#if sense.examples && sense.examples.length > 0}
            <div class="examples-block">
              <span class="sub-label">Examples:</span>
              <ul class="examples-list">
                {#each sense.examples as ex}
                  <li class="example-item">
                    <span class="ex-quote">“{ex.text}”</span>
                    <button
                      class="tts-mini-btn"
                      onclick={() => playAudio(ex.text, entry.language)}
                      title="Listen to example"
                      aria-label="Speak example"
                    >
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
                        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
                      </svg>
                    </button>
                    {#if ex.translation}
                      <span class="ex-trans">— {ex.translation}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            </div>
          {/if}

          {#if sense.synonyms && sense.synonyms.length > 0}
            <div class="relations-block">
              <span class="sub-label">Synonyms:</span>
              <span class="rel-text">{sense.synonyms.join(', ')}</span>
            </div>
          {/if}

          {#if sense.collocations && sense.collocations.length > 0}
            <div class="relations-block">
              <span class="sub-label">Commonly used with:</span>
              <span class="rel-text">{sense.collocations.join(', ')}</span>
            </div>
          {/if}

          <footer class="sense-source">
            <span>source: {sense.source_id}</span>
          </footer>
        </li>
      {/each}
    </ol>
  </section>
</article>

<style>
  .entry-view {
    padding: 0.5rem 0;
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
  .tts-btn {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.2rem 0.4rem;
    color: var(--accent);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    transition: background 0.15s, border-color 0.15s;
    vertical-align: middle;
  }
  .tts-btn:hover {
    background: var(--bg-hover, rgba(0, 0, 0, 0.05));
    border-color: var(--accent);
  }
  .entry-lemma {
    font-size: 1.75rem;
    font-weight: 700;
    margin: 0;
    color: var(--text-h);
  }
  .entry-lang {
    font-size: 0.85rem;
    color: var(--accent);
    font-weight: 600;
    text-transform: uppercase;
  }
  .entry-pos {
    font-size: 0.95rem;
    color: var(--text-muted);
    font-style: italic;
  }
  .pronunciations {
    margin-top: 0.25rem;
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
  .forms {
    margin-top: 0.5rem;
    font-size: 0.875rem;
  }
  .forms-label, .sub-label {
    font-weight: 600;
    color: var(--text-muted);
    font-size: 0.825rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-right: 0.25rem;
  }
  .form-badge {
    margin-right: 0.5rem;
    font-weight: 500;
  }
  .form-type {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .senses-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }
  .sense-item {
    border-left: 2px solid var(--border);
    padding-left: 1rem;
  }
  .sense-def {
    font-size: 1.05rem;
    line-height: 1.4;
    margin-bottom: 0.5rem;
  }
  .example-item {
    font-size: 0.95rem;
    line-height: 1.4;
    color: var(--text-h);
  }
  .tts-mini-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    vertical-align: middle;
    padding: 0 0.2rem;
    transition: color 0.15s;
  }
  .tts-mini-btn:hover {
    color: var(--accent);
  }
  .sense-num {
    font-weight: 700;
    color: var(--text-h);
    margin-right: 0.25rem;
  }
  .translations-block, .examples-block, .relations-block {
    margin-top: 0.5rem;
    font-size: 0.925rem;
  }
  .inline-list {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .tr-lang {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-right: 0.2rem;
  }
  .examples-list {
    list-style: none;
    padding: 0;
    margin: 0.25rem 0 0 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .example-item {
    font-style: italic;
    color: var(--text-h);
  }
  .ex-trans {
    font-style: normal;
    color: var(--text-muted);
    margin-left: 0.5rem;
  }
  .sense-source {
    margin-top: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    text-align: right;
  }
</style>
