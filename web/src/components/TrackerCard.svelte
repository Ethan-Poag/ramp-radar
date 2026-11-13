<script lang="ts">
  import type { Tracker } from '../lib/api';
  import { store } from '../lib/store.svelte';
  import { ago, fmtAlt, fmtDist, fmtEta, STATUS_LABEL } from '../lib/format';

  let { tracker }: { tracker: Tracker } = $props();
  const st = $derived(store.states[tracker.id]);
  const status = $derived(tracker.enabled ? (st?.status ?? 'not_seen') : 'disabled');
  const ap = $derived(store.airports[tracker.airport]);
  const err = $derived(store.errors[tracker.id]);
  let now = $state(Date.now());
  $effect(() => {
    const i = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(i);
  });
  const tail = $derived(st?.reg ?? (tracker.target.kind === 'reg' ? tracker.target.value : (st?.callsign ?? tracker.target.value.toUpperCase())));
</script>

<button class="card tcard" class:sel={store.selected === tracker.id} onclick={() => (store.selected = tracker.id)} style="--c:{tracker.color}">
  <div class="top">
    <span class="swatch"></span>
    <b class="tail">{tail}</b>
    <span class="label muted">{tracker.label}</span>
    <span class="pill {status}">{STATUS_LABEL[status]}</span>
  </div>
  <div class="grid">
    <div><span class="k">Airport</span><span class="v">{tracker.airport}{ap?.iata ? ` · ${ap.iata}` : ''}</span></div>
    <div><span class="k">Distance</span><span class="v">{fmtDist(st?.distance_nm, store.unit)}</span></div>
    <div><span class="k">ETA</span><span class="v">{fmtEta(st?.eta_min)}</span></div>
    <div><span class="k">Altitude</span><span class="v">{fmtAlt(st?.alt_ft)}</span></div>
  </div>
  <div class="foot small muted">
    <span>Radius {tracker.radius.value} {tracker.radius.unit}{tracker.ceiling_ft != null ? ` · below ${tracker.ceiling_ft.toLocaleString()} ft` : ''}</span>
    <span>
      {#if err}<span class="e" title={err}>⚠ data error</span>{' · '}{/if}last seen {ago(st?.last_seen, now)}{st?.provider ? ` · ${st.provider}` : ''}
    </span>
  </div>
</button>

<style>
  .tcard { text-align: left; width: 100%; padding: .8rem 1rem; display: block; border-left: 5px solid var(--c); }
  .tcard:hover { border-color: var(--c); }
  .tcard.sel { outline: 2px solid var(--accent); }
  .top { display: flex; align-items: center; gap: .55rem; margin-bottom: .55rem; flex-wrap: wrap; }
  .swatch { width: 10px; height: 10px; border-radius: 50%; background: var(--c); }
  .tail { font-size: 1.1rem; letter-spacing: .02em; }
  .label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: .5rem; }
  .k { display: block; font-size: .7rem; text-transform: uppercase; letter-spacing: .05em; color: var(--muted); }
  .v { font-weight: 600; font-variant-numeric: tabular-nums; }
  .foot { display: flex; justify-content: space-between; gap: .5rem; margin-top: .55rem; flex-wrap: wrap; }
  .e { color: var(--bad); }
  @media (max-width: 520px) { .grid { grid-template-columns: repeat(2, 1fr); } }
</style>
