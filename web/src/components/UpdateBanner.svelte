<script lang="ts">
  import { store } from '../lib/store.svelte';

  const show = $derived(
    store.info?.mode === 'desktop' && store.update.available && !store.update.dismissed && !!store.update.version,
  );
</script>

{#if show}
  <div class="banner" role="status">
    <span>Ramp Radar {store.update.version} is available — <button class="link" type="button" disabled={store.update.installing} onclick={() => store.installUpdate()}>{store.update.installing ? 'Updating…' : 'Update now'}</button></span>
    <button class="x" type="button" aria-label="Dismiss" onclick={() => (store.update.dismissed = true)}>×</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: .75rem;
    padding: .4rem 1rem;
    background: var(--panel2);
    border-bottom: 1px solid var(--line);
    font-size: .88rem;
    color: var(--text);
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    min-height: 0;
    color: var(--accent);
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .link:hover { border-color: transparent; }
  .x {
    border: 0;
    background: none;
    min-height: 0;
    padding: .1rem .35rem;
    color: var(--muted);
    font-size: 1.1rem;
    line-height: 1;
  }
</style>
