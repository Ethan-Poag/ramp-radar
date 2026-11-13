<script lang="ts">
  import { api } from '../lib/api';
  import { store } from '../lib/store.svelte';

  let { name, label, isSet = $bindable(), placeholder = '' }: { name: string; label: string; isSet: boolean; placeholder?: string } = $props();
  let v = $state('');
  async function save() {
    if (!v) return;
    try { await api.setSecret(name, v); isSet = true; v = ''; store.toast(`${label} stored`); } catch (e: any) { store.toast(e.message, 'err'); }
  }
  async function clear() {
    await api.deleteSecret(name); isSet = false; store.toast(`${label} removed`);
  }
</script>

<div class="field" style="margin:.6rem 0">
  <span class="small muted">{label} {#if isSet}<b class="set">● stored</b>{:else}<span>(not set)</span>{/if}</span>
  <div class="row" style="margin-top:.2rem">
    <input type="password" autocomplete="new-password" placeholder={isSet ? '•••••••• (enter new value to replace)' : placeholder} bind:value={v} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), save())} />
    <button type="button" class="shrink" onclick={save} disabled={!v}>Save</button>
    {#if isSet}<button type="button" class="shrink danger" onclick={clear}>Clear</button>{/if}
  </div>
</div>

<style>.set { color: var(--ok); }</style>
