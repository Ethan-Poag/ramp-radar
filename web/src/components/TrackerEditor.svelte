<script lang="ts">
  import { api, type ChannelKind, type Tracker } from '../lib/api';
  import { store, NEW } from '../lib/store.svelte';
  import AirportPicker from './AirportPicker.svelte';
  import { minToTime, timeToMin } from '../lib/format';

  const isNew = store.selected === NEW;
  const PALETTE = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#14b8a6', '#f97316'];

  function blank(): Tracker {
    return {
      id: '', label: '', target: { kind: 'reg', value: '' }, airport: '',
      radius: { value: 10, unit: store.unit === 'km' ? 'km' : store.unit },
      alert_enter: true, alert_exit: false, alert_landing: true, alert_takeoff: false,
      ceiling_ft: null, quiet_hours: null,
      channels: [...(store.settings?.default_channels ?? ['desktop', 'sound'])],
      color: PALETTE[store.trackers.length % PALETTE.length], enabled: true,
    };
  }
  let t = $state<Tracker>(isNew ? blank() : structuredClone($state.snapshot(store.selectedTracker!)));
  let ident = $state(t.target.value);
  let hint = $state('');
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout>;

  // Free-text identifier: let the server classify it (N-number / hex / callsign) and show the derived code.
  function onIdent() {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      if (!ident.trim()) return (hint = '');
      const r = await api.lookup(ident).catch(() => null);
      if (!r?.target) return (hint = '');
      t.target = r.target;
      hint =
        r.target.kind === 'reg' ? `Tail number${r.hex ? ` → Mode S ${r.hex.toUpperCase()}` : ' (non-US: matched by registration)'}`
        : r.target.kind === 'hex' ? `Mode S hex${r.n_number ? ` → ${r.n_number}` : ''}`
        : 'Callsign (flight number) — use hex if the aircraft has no fixed callsign';
    }, 200);
  }
  $effect(() => { if (ident) onIdent(); });

  const CHANNELS: [ChannelKind, string][] = [
    ['desktop', 'Desktop / browser'], ['sound', 'Sound'], ['ntfy', 'ntfy'], ['email', 'Email'], ['webhook', 'Webhook'], ['twilio', 'SMS (Twilio)'],
  ];
  function toggle(c: ChannelKind) {
    t.channels = t.channels.includes(c) ? t.channels.filter((x) => x !== c) : [...t.channels, c];
  }
  let quiet = $derived(t.quiet_hours !== null);

  async function save() {
    busy = true;
    try {
      if (!t.label.trim()) t.label = ident.toUpperCase();
      const saved = isNew ? await api.createTracker(t) : await api.updateTracker(t);
      await store.loadTrackers();
      store.selected = saved.id;
      store.toast(`Saved ${saved.label}`);
      api.pollNow().catch(() => {});
    } catch (e: any) {
      store.toast(e.message, 'err');
    } finally {
      busy = false;
    }
  }
  async function del() {
    if (!confirm(`Delete tracker "${t.label}" and its history?`)) return;
    await api.deleteTracker(t.id);
    await store.loadTrackers();
    store.selected = null;
  }
  async function test() {
    busy = true;
    try {
      const r = await api.testAlert(t.id);
      const failed = r.results.filter((x) => !x.ok);
      if (!r.results.length) store.toast('No notification channels selected for this tracker', 'err');
      else if (failed.length) store.toast(failed.map((f) => `${f.channel}: ${f.error}`).join('; '), 'err', 9000);
      else store.toast('Test alert sent');
    } catch (e: any) {
      store.toast(e.message, 'err');
    } finally { busy = false; }
  }
</script>

<form onsubmit={(e) => { e.preventDefault(); save(); }}>
  <h2>{isNew ? 'New tracker' : 'Tracker settings'}</h2>

  <label class="field">Label
    <input type="text" bind:value={t.label} placeholder="e.g. Dad's Bonanza" maxlength="80" />
  </label>

  <label class="field">Aircraft — tail number, Mode S hex, or callsign
    <input type="text" bind:value={ident} placeholder="N800BS, A4F123 or DAL42" autocapitalize="characters" spellcheck="false" required />
    {#if hint}<span class="small hint">{hint}</span>{/if}
  </label>
  <p class="small muted" style="margin:-.3rem 0 .4rem">Tail blocked from public feeds? Enter its Mode S hex instead.</p>

  <div class="field">Airport
    <AirportPicker bind:value={t.airport} />
  </div>

  <div class="row">
    <label class="field">Alert radius
      <input type="number" min="0.1" step="0.5" bind:value={t.radius.value} />
    </label>
    <label class="field shrink" style="width:6rem">Unit
      <select bind:value={t.radius.unit}><option value="nm">nm</option><option value="mi">mi</option><option value="km">km</option></select>
    </label>
  </div>

  <h3>Alert when</h3>
  <div>
    <label class="check"><input type="checkbox" bind:checked={t.alert_enter} /> enters radius</label>
    <label class="check"><input type="checkbox" bind:checked={t.alert_exit} /> leaves radius</label>
    <label class="check"><input type="checkbox" bind:checked={t.alert_landing} /> lands</label>
    <label class="check"><input type="checkbox" bind:checked={t.alert_takeoff} /> departs</label>
  </div>

  <label class="field">Altitude ceiling (ft) — ignore aircraft flying above
    <input type="number" min="0" step="500" placeholder="no limit" value={t.ceiling_ft ?? ''} oninput={(e) => (t.ceiling_ft = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))} />
  </label>

  <label class="check"><input type="checkbox" checked={quiet} onchange={(e) => (t.quiet_hours = e.currentTarget.checked ? { start_min: 22 * 60, end_min: 7 * 60 } : null)} /> Quiet hours (still logged, not sent)</label>
  {#if t.quiet_hours}
    <div class="row">
      <label class="field">From <input type="time" value={minToTime(t.quiet_hours.start_min)} onchange={(e) => (t.quiet_hours!.start_min = timeToMin(e.currentTarget.value))} /></label>
      <label class="field">To <input type="time" value={minToTime(t.quiet_hours.end_min)} onchange={(e) => (t.quiet_hours!.end_min = timeToMin(e.currentTarget.value))} /></label>
    </div>
  {/if}

  <h3>Notify via</h3>
  <div>
    {#each CHANNELS as [c, name]}
      <label class="check"><input type="checkbox" checked={t.channels.includes(c)} onchange={() => toggle(c)} /> {name}</label>
    {/each}
  </div>
  <p class="small muted">Configure ntfy, email, webhook and SMS in Settings.</p>

  <h3>Appearance</h3>
  <div class="swatches">
    {#each PALETTE as c}
      <button type="button" class="sw" class:on={t.color === c} style="background:{c}" aria-label="color {c}" onclick={() => (t.color = c)}></button>
    {/each}
  </div>
  <label class="check" style="margin-top:.8rem"><input type="checkbox" bind:checked={t.enabled} /> Enabled</label>

  <div class="actions">
    <button class="primary" type="submit" disabled={busy || !t.airport || !ident}>Save</button>
    {#if !isNew}
      <button type="button" onclick={test} disabled={busy}>Test alert</button>
      <button type="button" class="danger" onclick={del}>Delete</button>
    {:else}
      <button type="button" onclick={() => (store.selected = null)}>Cancel</button>
    {/if}
  </div>
</form>

<style>
  .hint { display: block; margin-top: .25rem; color: var(--accent); }
  .swatches { display: flex; gap: .45rem; flex-wrap: wrap; }
  .sw { width: 28px; height: 28px; min-height: 0; padding: 0; border-radius: 50%; border: 2px solid transparent; }
  .sw.on { border-color: var(--text); }
  .actions { display: flex; gap: .5rem; margin-top: 1.2rem; flex-wrap: wrap; }
</style>
