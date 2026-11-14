<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from './lib/store.svelte';
  import TrackersView from './components/TrackersView.svelte';
  import MapView from './components/MapView.svelte';
  import EventLog from './components/EventLog.svelte';
  import SettingsView from './components/SettingsView.svelte';
  import UpdateBanner from './components/UpdateBanner.svelte';

  let err = $state('');
  onMount(() => {
    store.init().catch((e) => (err = String(e.message ?? e)));
  });

  const tabs = [
    ['trackers', 'Trackers', '☰'],
    ['map', 'Map', '◎'],
    ['log', 'Log', '≡'],
    ['settings', 'Settings', '⚙'],
  ] as const;
  const healthy = $derived(store.providers.find((p) => p.ok));
  const failing = $derived(store.providers.length > 0 && !healthy && store.providers.some((p) => p.ok === false));
</script>

<div class="shell">
  <header>
    <div class="brand"><svg viewBox="0 0 64 64" width="26" height="26"><path fill="currentColor" d="M32 8c2 0 3 2 3 4v14l20 12v5l-20-6v10l6 5v4l-9-3-9 3v-4l6-5V37l-20 6v-5l20-12V12c0-2 1-4 3-4z"/></svg><span>Ramp Radar</span></div>
    <nav class="tabs desktop-only">
      {#each tabs as [id, label]}
        <button class:active={store.tab === id} onclick={() => (store.tab = id)}>{label}</button>
      {/each}
    </nav>
    <div class="status small" title="Data source health">
      <span class="dot" class:ok={healthy} class:bad={failing}></span>
      {#if healthy}{healthy.id}{:else if failing}data source error{:else}connecting…{/if}
      {#if !store.connected}<span class="muted"> · offline</span>{/if}
    </div>
  </header>
  <UpdateBanner />

  <main>
    {#if err}
      <div class="card" style="padding:1rem;margin:1rem">Could not reach the server: {err}</div>
    {:else if store.tab === 'trackers'}
      <TrackersView />
    {:else if store.tab === 'map'}
      <MapView />
    {:else if store.tab === 'log'}
      <EventLog />
    {:else}
      <SettingsView />
    {/if}
  </main>

  <nav class="bottombar mobile-only">
    {#each tabs as [id, label, icon]}
      <button class:active={store.tab === id} onclick={() => (store.tab = id)}><span>{icon}</span>{label}</button>
    {/each}
  </nav>

  <div class="toasts" aria-live="polite">
    {#each store.toasts as t (t.id)}
      <div class="toast {t.kind}">{t.text}</div>
    {/each}
  </div>
</div>

<style>
  .shell { height: 100%; display: flex; flex-direction: column; }
  header { display: flex; align-items: center; gap: 1rem; padding: .6rem 1rem; background: var(--panel); border-bottom: 1px solid var(--line); padding-top: max(.6rem, env(safe-area-inset-top)); }
  .brand { display: flex; align-items: center; gap: .5rem; font-weight: 700; color: var(--accent); white-space: nowrap; }
  .brand span { color: var(--text); }
  .tabs { display: flex; gap: .25rem; margin-left: 1rem; flex: 1; }
  .tabs button { border: 0; background: none; border-radius: 8px; color: var(--muted); font-weight: 600; }
  .tabs button.active { background: var(--bg); color: var(--text); }
  .status { margin-left: auto; display: flex; align-items: center; gap: .4rem; color: var(--muted); }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--muted); }
  .dot.ok { background: var(--ok); } .dot.bad { background: var(--bad); }
  main { flex: 1; min-height: 0; overflow: auto; }
  .bottombar { display: none; }
  .toasts { position: fixed; right: 1rem; top: 4rem; display: flex; flex-direction: column; gap: .5rem; z-index: 50; max-width: min(24rem, 90vw); }
  .toast { background: var(--panel); border: 1px solid var(--line); border-left: 4px solid var(--ok); padding: .6rem .9rem; border-radius: 8px; box-shadow: var(--shadow); font-size: .9rem; }
  .toast.err { border-left-color: var(--bad); } .toast.alert { border-left-color: var(--warn); font-weight: 600; }
  @media (max-width: 760px) {
    .bottombar { display: flex; background: var(--panel); border-top: 1px solid var(--line); padding-bottom: env(safe-area-inset-bottom); }
    .bottombar button { flex: 1; border: 0; background: none; border-radius: 0; display: flex; flex-direction: column; align-items: center; gap: 0; font-size: .72rem; color: var(--muted); min-height: 52px; }
    .bottombar button span { font-size: 1.2rem; line-height: 1.2; }
    .bottombar button.active { color: var(--accent); }
    .brand span { display: none; }
    .toasts { top: auto; bottom: 4.5rem; right: .5rem; left: .5rem; max-width: none; }
  }
</style>
