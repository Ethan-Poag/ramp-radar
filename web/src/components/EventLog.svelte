<script lang="ts">
  import { api } from '../lib/api';
  import { store } from '../lib/store.svelte';
  import { fmtAlt, fmtDist } from '../lib/format';

  let filter = $state('');
  let loaded = $state(false);
  $effect(() => {
    api.events(filter || undefined, 500).then((e) => { store.events = e; loaded = true; });
  });
  const KIND = { enter: 'Entered radius', exit: 'Left radius', landing: 'Landed (arrival)', takeoff: 'Departed' } as const;
</script>

<div class="page">
  <div class="bar">
    <h2 style="margin:0">Visit log</h2>
    <select bind:value={filter} style="max-width:14rem">
      <option value="">All trackers</option>
      {#each store.trackers as t}<option value={t.id}>{t.label}</option>{/each}
    </select>
    <a class="btn" href={`/api/events.csv${filter ? `?tracker=${filter}` : ''}`} download>Export CSV</a>
  </div>
  <div class="card tablewrap">
    <table>
      <thead><tr><th>Time</th><th>Tracker</th><th>Aircraft</th><th>Event</th><th>Airport</th><th>Distance</th><th>Alt</th></tr></thead>
      <tbody>
        {#each store.events as e (e.id)}
          <tr>
            <td>{new Date(e.ts).toLocaleString()}</td><td>{e.label}</td><td>{e.aircraft}</td>
            <td><span class="pill {e.kind === 'enter' ? 'within' : e.kind === 'landing' ? 'on_ground' : e.kind === 'takeoff' ? 'inbound' : 'outbound'}">{KIND[e.kind]}</span></td>
            <td>{e.airport}</td><td>{fmtDist(e.distance_nm, store.unit)}</td><td>{fmtAlt(e.alt_ft)}</td>
          </tr>
        {:else}
          <tr><td colspan="7" class="muted" style="padding:1.5rem;text-align:center">{loaded ? 'No events yet. Arrivals and departures appear here as they happen.' : 'Loading…'}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .page { padding: 1rem; max-width: 1200px; margin: 0 auto; }
  .bar { display: flex; gap: .75rem; align-items: center; margin-bottom: .75rem; flex-wrap: wrap; }
  .bar h2 { flex: 1; }
  .btn { padding: .5rem .9rem; border: 1px solid var(--line); border-radius: 8px; text-decoration: none; color: var(--text); background: var(--panel); }
  .tablewrap { overflow-x: auto; }
  td { white-space: nowrap; }
</style>
