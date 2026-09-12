<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type {
    EntryRecord,
    PackDiagnostics,
    PackInfo,
    SearchSuggestion,
  } from './lib/types';
  import { MOCK_ENTRIES, MOCK_PACKS, mockSuggest } from './lib/mockData';
  import EntryView from './lib/components/EntryView.svelte';

  let query = $state('');
  let suggestions = $state<SearchSuggestion[]>([]);
  let selectedEntry = $state<EntryRecord | null>(null);
  let statusMessage = $state('Loading dictionary packs…');
  let packs = $state<PackInfo[]>([]);
  let diagnostics = $state<PackDiagnostics | null>(null);
  let showPacksPanel = $state(false);
  let highlightIndex = $state(-1);
  let searchInput: HTMLInputElement | null = null;
  let debounceTimer: ReturnType<typeof setTimeout>;

  // Monotonic token so a slow response cannot overwrite a newer one. Worst-case content
  // search is far slower than the 80 ms debounce, so out-of-order replies were reachable.
  let searchToken = 0;

  // Capability probe, evaluated once. The previous implementation called invoke() and
  // sniffed the thrown error's text for '__TAURI_INTERNALS__' / 'not detected'; outside
  // Tauri the real error matches none of those strings, so the browser fallback never
  // ran and the UI showed a raw TypeError instead.
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  async function callBackend<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
    if (isTauri) return invoke<T>(cmd, args);

    // Browser preview. Results are clearly badged as sample data in the status bar so
    // they can never be mistaken for a real dictionary pack.
    switch (cmd) {
      case 'list_packs':
        return MOCK_PACKS as unknown as T;
      case 'pack_diagnostics':
        return {
          pack_dir: '(browser preview)',
          loaded: MOCK_PACKS.length,
          errors: [],
          settings_path: '',
        } as unknown as T;
      case 'toggle_pack':
        return true as unknown as T;
      case 'reorder_packs':
        return MOCK_PACKS as unknown as T;
      case 'suggest':
        return mockSuggest((args.query as string) || '') as unknown as T;
      case 'get_entry':
        return (MOCK_ENTRIES[(args.entryId as string) || ''] || null) as unknown as T;
      default:
        throw new Error(`Command not available in browser preview: ${cmd}`);
    }
  }

  function describeState() {
    if (!isTauri) {
      return 'Browser preview — showing built-in sample entries, not a dictionary pack.';
    }
    const enabled = packs.filter((p) => p.enabled);
    const entries = enabled.reduce((sum, p) => sum + p.entry_count, 0);
    const failed = diagnostics?.errors.length ?? 0;
    if (packs.length === 0) {
      const where = diagnostics?.pack_dir ?? 'packs';
      return `No dictionary packs found in ${where}. Build one with scripts/fetch_and_build_pack.py.`;
    }
    let msg = `${enabled.length} of ${packs.length} packs active · ${entries.toLocaleString()} entries`;
    if (failed > 0) msg += ` · ${failed} pack${failed === 1 ? '' : 's'} failed to load`;
    return msg;
  }

  async function refreshPacks() {
    try {
      packs = await callBackend<PackInfo[]>('list_packs');
      diagnostics = await callBackend<PackDiagnostics>('pack_diagnostics');
    } catch (err) {
      statusMessage = `Could not read pack list: ${err}`;
      return;
    }
    statusMessage = describeState();
  }

  /**
   * Moves a pack up or down the priority list. Position in the list decides which pack wins
   * an otherwise-identical match: `man` is an exact lemma in five languages at once, and
   * which one a reader wants first is a preference the engine cannot infer.
   */
  async function movePack(index: number, direction: -1 | 1) {
    const target = index + direction;
    if (target < 0 || target >= packs.length) return;
    const order = packs.map((p) => p.id);
    [order[index], order[target]] = [order[target], order[index]];
    try {
      packs = await callBackend<PackInfo[]>('reorder_packs', { packIds: order });
      statusMessage = describeState();
      if (query.trim()) runSearch();
    } catch (err) {
      statusMessage = `Could not reorder packs: ${err}`;
    }
  }

  async function togglePack(packId: string, currentEnabled: boolean) {
    try {
      await callBackend<boolean>('toggle_pack', { packId, enabled: !currentEnabled });
      await refreshPacks();
      if (query.trim()) runSearch();
    } catch (err) {
      statusMessage = `Could not switch that pack: ${err}`;
    }
  }

  function handleInput() {
    clearTimeout(debounceTimer);
    highlightIndex = -1;
    if (!query.trim()) {
      suggestions = [];
      statusMessage = describeState();
      return;
    }
    debounceTimer = setTimeout(runSearch, 80);
  }

  async function runSearch() {
    const token = ++searchToken;
    const q = query.trim();
    if (!q) return;
    try {
      const results = await callBackend<SearchSuggestion[]>('suggest', { query: q });
      if (token !== searchToken) return; // a newer query already went out
      suggestions = results;
      statusMessage = results.length
        ? `${results.length} match${results.length === 1 ? '' : 'es'} for “${q}”`
        : `No matches for “${q}”`;

      // Open the top hit only when it is genuinely an exact headword match. Otherwise
      // leave the choice to the reader rather than guessing.
      const top = results[0];
      if (top && top.match_type === 'lemma' && top.matched_term === q.toLowerCase()) {
        highlightIndex = 0;
        loadEntry(top.entry_id, token);
      } else {
        selectedEntry = null;
      }
    } catch (err) {
      if (token !== searchToken) return;
      statusMessage = `Search failed: ${err}`;
    }
  }

  async function loadEntry(entryId: string, token = searchToken) {
    try {
      const entry = await callBackend<EntryRecord | null>('get_entry', { entryId });
      if (token !== searchToken) return;
      selectedEntry = entry;
    } catch (err) {
      statusMessage = `Could not open that entry: ${err}`;
    }
  }

  function selectSuggestion(index: number) {
    const sugg = suggestions[index];
    if (!sugg) return;
    highlightIndex = index;
    loadEntry(sugg.entry_id);
  }

  function onSearchKeydown(event: KeyboardEvent) {
    if (!suggestions.length) return;
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        highlightIndex = (highlightIndex + 1) % suggestions.length;
        selectSuggestion(highlightIndex);
        break;
      case 'ArrowUp':
        event.preventDefault();
        highlightIndex = highlightIndex <= 0 ? suggestions.length - 1 : highlightIndex - 1;
        selectSuggestion(highlightIndex);
        break;
      case 'Enter':
        event.preventDefault();
        selectSuggestion(highlightIndex < 0 ? 0 : highlightIndex);
        break;
      case 'Escape':
        event.preventDefault();
        suggestions = [];
        highlightIndex = -1;
        break;
    }
  }

  function onGlobalKeydown(event: KeyboardEvent) {
    // Ctrl/Cmd+L focuses the search box, as the UI contract specifies.
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
      event.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }

  onMount(() => {
    refreshPacks();
    searchInput?.focus();
  });
</script>

<svelte:window onkeydown={onGlobalKeydown} />

<main class="app-layout">
  <header class="app-header">
    <div class="header-top">
      <input
        id="golddig-search"
        class="search-input"
        type="search"
        placeholder="Type a word, phrase, or pattern (l?ght, l*t)…"
        bind:value={query}
        bind:this={searchInput}
        oninput={handleInput}
        onkeydown={onSearchKeydown}
        role="combobox"
        aria-label="Dictionary search"
        aria-expanded={suggestions.length > 0}
        aria-controls="golddig-suggestions"
        aria-activedescendant={highlightIndex >= 0 ? `golddig-sugg-${highlightIndex}` : undefined}
        aria-autocomplete="list"
        autocomplete="off"
        spellcheck="false"
      />
      <button
        class="packs-btn"
        onclick={() => (showPacksPanel = !showPacksPanel)}
        aria-expanded={showPacksPanel}
        aria-controls="golddig-packs"
      >
        Packs ({packs.filter((p) => p.enabled).length}/{packs.length})
      </button>
    </div>

    {#if showPacksPanel}
      <div class="packs-panel" id="golddig-packs">
        <div class="packs-header">
          <h2>Dictionary packs</h2>
          <button class="close-btn" onclick={() => (showPacksPanel = false)} aria-label="Close pack list">
            ✕
          </button>
        </div>

        {#if packs.length === 0}
          <p class="no-packs">
            No packs loaded. Golddig looked in <code>{diagnostics?.pack_dir ?? 'packs'}</code>.
          </p>
        {:else}
          <p class="packs-hint">
            Order decides which pack wins when two entries match equally well. Use the arrows.
            {#if isTauri && diagnostics && !diagnostics.settings_path}
              <strong>Changes apply to this session only — no writable data directory was
              found, so they cannot be saved.</strong>
            {/if}
          </p>
          <ol class="packs-list">
            {#each packs as pack, index (pack.id)}
              <li class="pack-item">
                <span class="pack-rank">{index + 1}</span>
                <label class="pack-label">
                  <input
                    type="checkbox"
                    checked={pack.enabled}
                    onchange={() => togglePack(pack.id, pack.enabled)}
                  />
                  <span class="pack-details">
                    <span class="pack-name">{pack.name}</span>
                    <span class="pack-meta">
                      v{pack.version} · {pack.entry_count.toLocaleString()} entries ·
                      {pack.languages.join(', ')}
                    </span>
                  </span>
                </label>
                <span class="pack-move">
                  <button
                    onclick={() => movePack(index, -1)}
                    disabled={index === 0}
                    title="Rank {pack.name} higher"
                    aria-label="Move {pack.name} up"
                  >↑</button>
                  <button
                    onclick={() => movePack(index, 1)}
                    disabled={index === packs.length - 1}
                    title="Rank {pack.name} lower"
                    aria-label="Move {pack.name} down"
                  >↓</button>
                </span>
              </li>
            {/each}
          </ol>
        {/if}

        {#if diagnostics && diagnostics.errors.length > 0}
          <ul class="pack-errors">
            {#each diagnostics.errors as err (err.path)}
              <li><strong>{err.path}</strong> — {err.message}</li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

    <p class="status-bar" role="status">{statusMessage}</p>
  </header>

  <div class="main-body">
    {#if suggestions.length > 0}
      <nav class="suggestions-sidebar" aria-label="Search results">
        <ul class="suggestions-list" id="golddig-suggestions" role="listbox">
          {#each suggestions as sugg, index (sugg.entry_id)}
            <li role="none">
              <button
                id="golddig-sugg-{index}"
                class="suggestion-btn"
                class:active={index === highlightIndex}
                role="option"
                aria-selected={index === highlightIndex}
                onclick={() => selectSuggestion(index)}
              >
                <span class="sugg-lemma">{sugg.lemma}</span>
                <span class="sugg-meta">
                  <span class="sugg-lang">{sugg.language}</span>
                  {#if sugg.match_type !== 'lemma'}
                    <span class="sugg-type">{sugg.match_type}</span>
                  {/if}
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </nav>
    {/if}

    <section class="content-pane" aria-live="polite">
      {#if selectedEntry}
        <EntryView entry={selectedEntry} />
      {:else if suggestions.length > 0}
        <p class="empty-state">Select a result to read its entry.</p>
      {:else}
        <p class="empty-state">Type a word to search your local dictionary packs.</p>
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
    gap: 0.75rem;
  }
  .app-header {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.75rem;
  }
  .header-top {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }
  .search-input {
    flex: 1;
    min-width: 0;
    font-size: 1.1rem;
  }
  .packs-btn {
    background: var(--code-bg);
    border: 1px solid var(--border);
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
    padding: 0.75rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface);
  }
  .packs-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .packs-header h2 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
  }
  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
  }
  .packs-list,
  .pack-errors {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .packs-hint {
    margin: 0 0 0.5rem;
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .pack-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
  }
  .pack-rank {
    flex: none;
    min-width: 1.1rem;
    font-size: 0.72rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    padding-top: 0.2rem;
  }
  .pack-move {
    margin-left: auto;
    display: flex;
    gap: 0.15rem;
    flex: none;
  }
  .pack-move button {
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    cursor: pointer;
    line-height: 1;
    padding: 0.1rem 0.3rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .pack-move button:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .pack-move button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .pack-label {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    cursor: pointer;
    min-width: 0;
  }
  .pack-label input[type='checkbox'] {
    margin-top: 0.3rem;
    flex: none;
  }
  .pack-details {
    display: flex;
    flex-direction: column;
  }
  .pack-name {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .pack-meta,
  .no-packs {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .no-packs {
    margin: 0.25rem 0;
  }
  .pack-errors {
    margin-top: 0.6rem;
    padding-top: 0.6rem;
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
    color: #b4342f;
  }
  .status-bar {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .main-body {
    display: flex;
    flex: 1;
    min-height: 0;
    gap: 1.25rem;
  }
  .suggestions-sidebar {
    width: 210px;
    flex: none;
    border-right: 1px solid var(--border);
    padding-right: 0.6rem;
    overflow-y: auto;
  }
  .suggestions-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .suggestion-btn {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 0.35rem 0.5rem;
    border-radius: 3px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.5rem;
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
    overflow-wrap: anywhere;
  }
  .sugg-meta {
    font-size: 0.7rem;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .sugg-lang {
    text-transform: uppercase;
  }
  .sugg-type {
    font-style: italic;
  }
  .content-pane {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
  }
  .empty-state {
    color: var(--text-muted);
    font-style: italic;
    padding-top: 2rem;
    text-align: center;
  }

  @media (max-width: 640px) {
    .main-body {
      flex-direction: column;
    }
    .suggestions-sidebar {
      width: 100%;
      max-height: 9rem;
      border-right: none;
      border-bottom: 1px solid var(--border);
      padding-right: 0;
      padding-bottom: 0.5rem;
    }
  }
</style>
