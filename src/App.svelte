<svelte:head>
  <title>Golddig</title>
  <meta
    name="description"
    content="Golddig is a fast, local-first dictionary lookup application."
  />
</svelte:head>

<main>
  <header>
    <p class="eyebrow">Local dictionary</p>
    <h1>Golddig</h1>
    <p class="introduction">Fast, text-first lookup from dictionaries stored on your device.</p>
  </header>

  <section aria-labelledby="search-heading">
    <h2 id="search-heading">Dictionary search</h2>
    <label for="query">Search</label>
    <input
      id="query"
      name="query"
      type="search"
      placeholder="Type a word"
      autocomplete="off"
      spellcheck="false"
      aria-describedby="pack-status"
    />
    <p id="pack-status" role="status">The local dictionary pack is not loaded.</p>
  </section>
</main>
