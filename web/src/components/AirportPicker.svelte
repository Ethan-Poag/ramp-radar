<script lang="ts">
  import { api, type Airport } from '../lib/api';
  import { store } from '../lib/store.svelte';

  let { value = $bindable() }: { value: string } = $props();
  let q = $state('');
  let results = $state<Airport[]>([]);
  let open = $state(false);
  let timer: ReturnType<typeof setTimeout>;
  const current = $derived(store.airports[value]);
  $effect(() => {
    if (value) store.ensureAirport(value);
  });

  function onInput() {
    clearTimeout(timer);
    open = true;
    timer = setTimeout(async () => {
      results = q.trim() ? await api.airports(q).catch(() => []) : [];
    }, 150);
  }
  function pick(a: Airport) {
    store.airports[a.ident] = a;
    value = a.ident;
    q = '';
    results = [];
    open = false;
  }
</script>

<div class="wrap">
  <input type="text" placeholder="Search ICAO, IATA, name or city…" bind:value={q} oninput={onInput} onfocus={() => (open = true)} onblur={() => setTimeout(() => (open = false), 150)} autocomplete="off" />
  {#if open && results.length}
    <ul class="card">
      {#each results as a}
        <li><button type="button" onmousedown={() => pick(a)}><b>{a.ident}</b>{#if a.iata} <span class="muted">{a.iata}</span>{/if} — {a.name}<span class="muted small"> · {a.city ?? ''} {a.country}</span></button></li>
      {/each}
    </ul>
  {/if}
  {#if current}
    <div class="sel small"><b>{current.ident}</b> {current.name}{#if current.elevation_ft != null} · {current.elevation_ft} ft{/if}</div>
  {:else if value}
    <div class="sel small">{value}</div>
  {/if}
</div>

<style>
  .wrap { position: relative; }
  ul { position: absolute; z-index: 20; left: 0; right: 0; margin: 2px 0 0; padding: 4px; list-style: none; max-height: 18rem; overflow: auto; }
  li button { width: 100%; text-align: left; border: 0; background: none; padding: .45rem .5rem; border-radius: 6px; min-height: 0; }
  li button:hover { background: var(--bg); }
  .sel { margin-top: .3rem; color: var(--text); }
</style>
