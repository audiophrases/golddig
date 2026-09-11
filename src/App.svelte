<script lang="ts">
  import { onMount } from 'svelte';
  import type { EntryRecord, SearchSuggestion } from './lib/types';
  import EntryView from './lib/components/EntryView.svelte';

  let query = $state('');
  let suggestions = $state<SearchSuggestion[]>([]);
  let selectedEntry = $state<EntryRecord | null>(null);
  let statusMessage = $state('Ready. Local dictionary pack loaded.');
  let debounceTimer: ReturnType<typeof setTimeout>;

  async function callTauri<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
    // Check if running inside Tauri window
    const w = window as unknown as {
      __TAURI__?: { core?: { invoke: (cmd: string, args: unknown) => Promise<T> } };
      __TAURI_INTERNALS__?: { invoke: (cmd: string, args: unknown) => Promise<T> };
    };
    if (w.__TAURI__?.core?.invoke) {
      return await w.__TAURI__.core.invoke(cmd, args);
    }
    if (w.__TAURI_INTERNALS__?.invoke) {
      return await w.__TAURI_INTERNALS__.invoke(cmd, args);
    }
    // Fallback for browser testing or mock mode
    throw new Error('Tauri core runtime not detected');
  }

  function handleInput() {
    clearTimeout(debounceTimer);
    if (!query.trim()) {
      suggestions = [];
      return;
    }

    debounceTimer = setTimeout(async () => {
      try {
        statusMessage = 'Searching...';
        const res = await callTauri<SearchSuggestion[]>('suggest', { query: query.trim() });
        suggestions = res;
        statusMessage = `${res.length} matches found`;
        if (res.length > 0) {
          // Auto-load top exact match
          loadEntry(res[0].entry_id);
        }
      } catch (err) {
        statusMessage = `Search error: ${err}`;
      }
    }, 80);
  }

  async function loadEntry(entryId: string) {
    try {
      const entry = await callTauri<EntryRecord | null>('get_entry', { entryId });
      selectedEntry = entry;
    } catch (err) {
      statusMessage = `Failed to load entry: ${err}`;
    }
  }

  onMount(() => {
    // Initial probe
  });
</script>

<main class="app-layout">
  <header class="app-header">
    <div class="search-bar">
      <input
        type="search"
        placeholder="Type a word (en, ca, es, fr, de)..."
        bind:value={query}
        oninput={handleInput}
        aria-label="Dictionary search"
      />
    </div>
    <div class="status-bar" role="status">
      <span>{statusMessage}</span>
    </div>
  </header>

  <div class="main-body">
    {#if suggestions.length > 0}
      <aside class="suggestions-sidebar">
        <ul class="suggestions-list">
          {#each suggestions as sugg}
            <li>
              <button
                class="suggestion-btn"
                class:active={selectedEntry?.id === sugg.entry_id}
                onclick={() => loadEntry(sugg.entry_id)}
              >
                <span class="sugg-lemma">{sugg.lemma}</span>
                <span class="sugg-meta">
                  <span class="sugg-lang">{sugg.language}</span>
                  {#if sugg.match_type !== 'lemma'}
                    <span class="sugg-type">[{sugg.match_type}]</span>
                  {/if}
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </aside>
    {/if}

    <section class="content-pane">
      {#if selectedEntry}
        <EntryView entry={selectedEntry} />
      {:else}
        <div class="empty-state">
          <p>No entry displayed. Type a query to search the local dictionary pack.</p>
        </div>
      {/if}
    </section>
  </div>
</main>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    max-width: 960px;
    margin: 0 auto;
    padding: 1rem;
    box-sizing: border-box;
  }
  .app-header {
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.75rem;
  }
  .search-bar input {
    width: 100%;
    font-size: 1.15rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text-h);
    box-sizing: border-box;
  }
  .search-bar input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .status-bar {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-top: 0.35rem;
  }
  .main-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    gap: 1.5rem;
    margin-top: 1rem;
  }
  .suggestions-sidebar {
    width: 220px;
    border-right: 1px solid var(--border);
    padding-right: 0.75rem;
    overflow-y: auto;
  }
  .suggestions-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .suggestion-btn {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 0.4rem 0.5rem;
    border-radius: 3px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    color: var(--text);
  }
  .suggestion-btn:hover {
    background: var(--code-bg);
  }
  .suggestion-btn.active {
    background: var(--accent-bg);
    color: var(--accent);
    font-weight: 600;
  }
  .sugg-lemma {
    font-size: 0.95rem;
  }
  .sugg-meta {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .sugg-lang {
    text-transform: uppercase;
  }
  .content-pane {
    flex: 1;
    overflow-y: auto;
    padding-right: 0.5rem;
  }
  .empty-state {
    color: var(--text-muted);
    font-style: italic;
    padding-top: 2rem;
    text-align: center;
  }
</style>
