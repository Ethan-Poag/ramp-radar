<script lang="ts">
  import { store, NEW } from '../lib/store.svelte';
  import TrackerCard from './TrackerCard.svelte';
  import TrackerEditor from './TrackerEditor.svelte';

  const editing = $derived(store.selected !== null);
</script>

<div class="split" class:editing>
  <section class="left card">
    {#if editing}
      <button class="back mobile-only" onclick={() => (store.selected = null)}>← Trackers</button>
      {#key store.selected}
        <TrackerEditor />
      {/key}
    {:else}
      <div class="empty">
        <h2>Tracker settings</h2>
        <p class="muted">Select a tracker card on the right to edit it, or create a new one.</p>
        <button class="primary" onclick={() => (store.selected = NEW)}>+ New tracker</button>
      </div>
    {/if}
  </section>

  <section class="right">
    <div class="bar">
      <h2 style="margin:0">Trackers <span class="muted small">({store.trackers.length})</span></h2>
      <button class="primary" onclick={() => (store.selected = NEW)}>+ New tracker</button>
    </div>
    {#each store.trackers as t (t.id)}
      <TrackerCard tracker={t} />
    {:else}
      <div class="card hint">
        <b>No trackers yet.</b>
        <p class="muted" style="margin:.4rem 0 0">Add an aircraft (tail number, Mode S hex or callsign), pick an airport and a radius, and you'll be alerted when it gets close.</p>
      </div>
    {/each}
  </section>
</div>

<style>
  .split { display: grid; grid-template-columns: minmax(300px, 420px) 1fr; gap: 1rem; padding: 1rem; align-items: start; max-width: 1400px; margin: 0 auto; }
  .left { padding: 1rem 1.1rem; position: sticky; top: 1rem; max-height: calc(100vh - 6rem); overflow: auto; }
  .right { display: flex; flex-direction: column; gap: .7rem; min-width: 0; }
  .bar { display: flex; justify-content: space-between; align-items: center; }
  .hint { padding: 1.2rem; }
  .empty { text-align: center; padding: 2rem 0; }
  .back { margin-bottom: .6rem; }
  @media (max-width: 760px) {
    .split { grid-template-columns: 1fr; padding: .6rem; }
    .left { position: static; max-height: none; }
    .split:not(.editing) .left { display: none; }
    .split.editing .right { display: none; }
  }
</style>
