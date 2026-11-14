<script lang="ts">
  import { api, type ChannelKind, type ProviderConfig, type Settings } from '../lib/api';
  import { applyTheme, store } from '../lib/store.svelte';
  import SecretField from './SecretField.svelte';

  let s = $state<Settings>(structuredClone($state.snapshot(store.settings!)));
  let secrets = $state<Record<string, boolean>>({});
  let blocklist = $state(s.blocklist.join('\n'));
  let busy = $state(false);
  api.secrets().then((x) => (secrets = x));

  const NAMES: Record<string, string> = {
    local: 'Local receiver (readsb / dump1090 / tar1090)',
    adsb_lol: 'adsb.lol — free community API (ODbL)',
    airplanes_live: 'airplanes.live — free community API',
    adsb_fi: 'adsb.fi — free community API',
    open_sky: 'OpenSky Network (OAuth client credentials)',
    adsb_exchange: 'ADS-B Exchange (your own RapidAPI key)',
    mock: 'Simulated traffic (demo / testing)',
  };

  async function save() {
    busy = true;
    try {
      s.blocklist = blocklist.split(/[\n,]+/).map((x) => x.trim()).filter(Boolean);
      store.settings = await api.saveSettings($state.snapshot(s) as Settings);
      applyTheme(s.theme);
      store.toast('Settings saved');
    } catch (e: any) {
      store.toast(e.message, 'err');
    } finally { busy = false; }
  }
  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= s.providers.length) return;
    [s.providers[i], s.providers[j]] = [s.providers[j], s.providers[i]];
  }
  async function test(c: ChannelKind) {
    await save();
    try {
      const r = await api.testChannel(c);
      const x = r.results[0];
      store.toast(x?.ok ? `${c}: test sent` : `${c}: ${x?.error ?? 'failed'}`, x?.ok ? 'ok' : 'err', 8000);
    } catch (e: any) { store.toast(e.message, 'err'); }
  }
  async function askNotify() {
    if (!('Notification' in window)) return store.toast('This browser has no notification support', 'err');
    const p = await Notification.requestPermission();
    store.toast(`Browser notifications: ${p}`, p === 'granted' ? 'ok' : 'err');
  }
  const HEALTH_ID: Record<string, string> = { local: 'local', adsb_lol: 'adsb.lol', airplanes_live: 'airplanes.live', adsb_fi: 'adsb.fi', open_sky: 'opensky', adsb_exchange: 'adsbexchange', mock: 'mock' };
  const pcfg = (p: ProviderConfig) => p as any;
</script>

<form class="page" onsubmit={(e) => { e.preventDefault(); save(); }}>
  <div class="bar"><h2 style="margin:0">Settings</h2><button class="primary" type="submit" disabled={busy}>Save settings</button></div>

  <section class="card">
    <h3 style="margin-top:0">General</h3>
    <div class="row">
      <label class="field">Distance units <select bind:value={s.distance_unit}><option value="nm">Nautical miles</option><option value="mi">Statute miles</option><option value="km">Kilometres</option></select></label>
      <label class="field">Theme <select bind:value={s.theme} onchange={() => applyTheme(s.theme)}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
      <label class="field">Poll interval (s) <input type="number" min="5" max="3600" bind:value={s.poll_interval_s} /></label>
    </div>
    <p class="small muted">Community APIs are free but rate-limited. 15 s or more is polite; a local receiver can go as low as 5 s.</p>
  </section>

  <section class="card">
    <h3 style="margin-top:0">Data sources <span class="muted" style="text-transform:none;letter-spacing:0">(tried top to bottom; failures fall through to the next)</span></h3>
    {#each s.providers as p, i}
      <div class="prov">
        <div class="row" style="align-items:center">
          <label class="check" style="flex:1"><input type="checkbox" bind:checked={s.providers[i].enabled} /> <b>{NAMES[p.type]}</b>
            {#each store.providers.filter((h) => h.id === HEALTH_ID[p.type]) as h}
              <span class="small" style="color:{h.ok === false ? 'var(--bad)' : h.ok ? 'var(--ok)' : 'var(--muted)'}">● {h.ok === false ? (h.last_error ?? 'error') : h.ok ? 'ok' : ''}</span>
            {/each}
          </label>
          <button type="button" class="shrink" onclick={() => move(i, -1)} aria-label="move up" disabled={i === 0}>↑</button>
          <button type="button" class="shrink" onclick={() => move(i, 1)} aria-label="move down" disabled={i === s.providers.length - 1}>↓</button>
        </div>
        {#if p.type === 'local'}
          <input type="url" placeholder="http://raspberrypi.local/tar1090/data/aircraft.json" bind:value={() => pcfg(p).url, (v) => (pcfg(p).url = v)} />
        {:else if p.type === 'open_sky'}
          <input type="text" placeholder="OAuth client id (optional — anonymous access is very limited)" bind:value={() => pcfg(p).client_id, (v) => (pcfg(p).client_id = v)} />
          <SecretField name="opensky_client_secret" label="OpenSky client secret" bind:isSet={secrets['opensky_client_secret']} />
        {:else if p.type === 'adsb_exchange'}
          <SecretField name="adsbx_rapidapi_key" label="RapidAPI key" bind:isSet={secrets['adsbx_rapidapi_key']} />
        {/if}
      </div>
    {/each}
  </section>

  <section class="card">
    <h3 style="margin-top:0">Notifications</h3>
    <label class="check"><input type="checkbox" bind:checked={s.notifications.sound} /> In-app sound</label>
    <label class="check"><input type="checkbox" bind:checked={s.notifications.desktop} /> Desktop / browser notifications</label>
    {#if store.info?.mode === 'server'}<button type="button" onclick={askNotify}>Allow browser notifications</button>{/if}
    <button type="button" onclick={() => test('desktop')}>Test</button>

    <h3>ntfy.sh</h3>
    <label class="check"><input type="checkbox" bind:checked={s.notifications.ntfy.enabled} /> Enabled</label>
    <div class="row">
      <label class="field">Server <input type="url" bind:value={s.notifications.ntfy.server} placeholder="https://ntfy.sh" /></label>
      <label class="field">Topic <input type="text" bind:value={s.notifications.ntfy.topic} placeholder="choose-a-long-random-topic" /></label>
    </div>
    <SecretField name="ntfy_token" label="Access token (optional)" bind:isSet={secrets['ntfy_token']} />
    <button type="button" onclick={() => test('ntfy')}>Send test</button>
    <p class="small muted">Anyone who knows a public ntfy topic name can read it — pick something unguessable, or self-host.</p>

    <h3>Email (SMTP)</h3>
    <label class="check"><input type="checkbox" bind:checked={s.notifications.email.enabled} /> Enabled</label>
    <div class="row">
      <label class="field">Host <input type="text" bind:value={s.notifications.email.host} placeholder="smtp.example.com" /></label>
      <label class="field shrink" style="width:6rem">Port <input type="number" bind:value={s.notifications.email.port} /></label>
      <label class="field">Security <select bind:value={s.notifications.email.security}><option value="starttls">STARTTLS</option><option value="tls">TLS</option><option value="none">None</option></select></label>
    </div>
    <div class="row">
      <label class="field">Username <input type="text" bind:value={s.notifications.email.username} autocomplete="off" /></label>
      <label class="field">From <input type="text" bind:value={s.notifications.email.from} placeholder="Ramp Radar <me@example.com>" /></label>
      <label class="field">To <input type="email" bind:value={s.notifications.email.to} /></label>
    </div>
    <SecretField name="smtp_password" label="SMTP password" bind:isSet={secrets['smtp_password']} />
    <button type="button" onclick={() => test('email')}>Send test</button>

    <h3>Webhook</h3>
    <label class="check"><input type="checkbox" bind:checked={s.notifications.webhook.enabled} /> Enabled</label>
    <SecretField name="webhook_url" label="Webhook URL (POSTs JSON: title, message, text)" bind:isSet={secrets['webhook_url']} placeholder="https://hooks.example.com/…" />
    <button type="button" onclick={() => test('webhook')}>Send test</button>

    <h3>SMS via Twilio (your own account)</h3>
    <label class="check"><input type="checkbox" bind:checked={s.notifications.twilio.enabled} /> Enabled</label>
    <div class="row">
      <label class="field">Account SID <input type="text" bind:value={s.notifications.twilio.account_sid} autocomplete="off" /></label>
      <label class="field">From <input type="text" bind:value={s.notifications.twilio.from} placeholder="+15551230000" /></label>
      <label class="field">To <input type="text" bind:value={s.notifications.twilio.to} placeholder="+15559870000" /></label>
    </div>
    <SecretField name="twilio_auth_token" label="Auth token" bind:isSet={secrets['twilio_auth_token']} />
    <button type="button" onclick={() => test('twilio')}>Send test</button>
  </section>

  <section class="card">
    <h3 style="margin-top:0">Privacy</h3>
    <label class="field">Block list — aircraft that must never be tracked or shown (hex, registration or callsign; one per line)
      <textarea rows="3" bind:value={blocklist} placeholder="a1b2c3&#10;N12345"></textarea>
    </label>
    <p class="small muted">Nothing is published anywhere unless you configure a notification channel. Aircraft enrolled in the FAA's LADD or PIA programs are filtered out of public feeds by design; Ramp Radar does not try to bypass that. Secrets (tokens, passwords) are stored in the OS keychain on desktop, or in a 0600 file on the server, and are never shown again after saving.</p>
  </section>

  <section class="card small muted">
    <h3 style="margin-top:0">About &amp; data attribution</h3>
    Ramp Radar {store.info?.version} · MIT licence. Map © OpenStreetMap contributors. Airports: OurAirports (public domain). Live traffic via adsb.lol is ODbL-licensed (© adsb.lol contributors); airplanes.live and adsb.fi data are subject to their own terms. Please respect each service's rate limits.
  </section>
</form>

<style>
  .page { padding: 1rem; max-width: 900px; margin: 0 auto; display: flex; flex-direction: column; gap: 1rem; }
  .bar { display: flex; justify-content: space-between; align-items: center; }
  section { padding: 1rem 1.2rem; }
  .prov { padding: .5rem 0; border-top: 1px solid var(--line); }
  .prov:first-of-type { border-top: 0; }
  textarea { width: 100%; padding: .5rem .6rem; border: 1px solid var(--line); border-radius: 8px; background: var(--panel2); color: inherit; font: inherit; margin-top: .2rem; }
</style>
