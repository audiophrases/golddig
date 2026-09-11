<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { EntryRecord, PackInfo, SearchSuggestion } from './lib/types';
  import { MOCK_ENTRIES, MOCK_PACKS, mockSuggest } from './lib/mockData';
  import EntryView from './lib/components/EntryView.svelte';

  let query = $state('');
  let suggestions = $state<SearchSuggestion[]>([]);
  let selectedEntry = $state<EntryRecord | null>(null);
  let statusMessage = $state('Ready. Local dictionary packs loaded.');
  let packs = $state<PackInfo[]>([]);
  let showPacksModal = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout>;

  async function callTauri<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
    // 1. Direct invoke from @tauri-apps/api/core
    try {
      return await invoke<T>(cmd, args);
    } catch (err: unknown) {
      // If invoke is rejected because Tauri runtime is truly absent (e.g. running in standard browser)
      // or window.__TAURI_INTERNALS__ is not present
      const errStr = String(err);
      if (
        errStr.includes('__TAURI_INTERNALS__') ||
        errStr.includes('not detected') ||
        errStr.includes('is not a function')
      ) {
        // Fall through to mock fallback
      } else {
        // If it was a real backend error from Rust, rethrow
        throw err;
      }
    }

    // 2. Graceful browser preview fallback (for testing in Chrome / edge / dev server without Tauri wrapper)
    if (cmd === 'list_packs') {
      return MOCK_PACKS as unknown as T;
    }
    if (cmd === 'toggle_pack') {
      return true as unknown as T;
    }
    if (cmd === 'suggest') {
      const q = (args.query as string) || '';
      return mockSuggest(q) as unknown as T;
    }
    if (cmd === 'get_entry') {
      const entryId = (args.entryId as string) || '';
      return (MOCK_ENTRIES[entryId] || null) as unknown as T;
    }

    throw new Error(`Command not supported in browser preview: ${cmd}`);
  }

  async function refreshPacks() {
    try {
      const p = await callTauri<PackInfo[]>('list_packs');
      packs = p;
    } catch {
      // In web browser dev/preview mode without native Tauri
    }
  }

  async function togglePack(packId: string, currentEnabled: boolean) {
    try {
      await callTauri<boolean>('toggle_pack', { packId, enabled: !currentEnabled });
      await refreshPacks();
      if (query.trim()) {
        handleInput();
      }
    } catch (err) {
      statusMessage = `Failed to toggle pack: ${err}`;
    }
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
    refreshPacks();
  });
</script>

<main class="app-layout">
  <header class="app-header">
    <div class="header-top">
      <div class="search-bar">
        <input
          type="search"
          placeholder="Type a word, phrase, or pattern (e.g. l?ght, l*t)..."
          bind:value={query}
          oninput={handleInput}
          aria-label="Dictionary search"
        />
      </div>
      <button
        class="packs-btn"
        onclick={() => (showPacksModal = !showPacksModal)}
        aria-label="Manage dictionary packs"
      >
        Packs ({packs.filter((p) => p.enabled).length}/{packs.length})
      </button>
    </div>

    {#if showPacksModal}
      <div class="packs-panel">
        <div class="packs-header">
          <h3>Active Dictionary Packs</h3>
          <button class="close-btn" onclick={() => (showPacksModal = false)}>✕</button>
        </div>
        {#if packs.length === 0}
          <p class="no-packs">No external packs loaded. Base fixture active.</p>
        {:else}
          <ul class="packs-list">
            {#each packs as p}
              <li class="pack-item">
                <label class="pack-label">
                  <input
                    type="checkbox"
                    checked={p.enabled}
                    onchange={() => togglePack(p.id, p.enabled)}
                  />
                  <div class="pack-details">
                    <span class="pack-name">{p.name} (v{p.version})</span>
                    <span class="pack-meta">
                      {p.entry_count} entries · [{p.languages.join(', ')}]
                    </span>
                  </div>
                </label>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

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
  .header-top {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }
  .search-bar {
    flex: 1;
  }
  .packs-btn {
    background: var(--code-bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 0.5rem 0.85rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    white-space: nowrap;
  }
  .packs-btn:hover {
    border-color: var(--accent);
  }
  .packs-panel {
    margin-top: 0.75rem;
    padding: 0.75rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--code-bg);
  }
  .packs-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .packs-header h3 {
    margin: 0;
    font-size: 0.95rem;
  }
  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
  }
  .packs-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .pack-item {
    display: flex;
    align-items: center;
  }
  .pack-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }
  .pack-details {
    display: flex;
    flex-direction: column;
  }
  .pack-name {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .pack-meta {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .no-packs {
    font-size: 0.85rem;
    color: var(--text-muted);
    margin: 0.25rem 0;
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
