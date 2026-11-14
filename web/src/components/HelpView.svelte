<script lang="ts">
  const toc = [
    ['start', 'Quick start'],
    ['sources', 'Data sources'],
    ['notify', 'Notifications'],
    ['privacy', 'Privacy'],
    ['server', 'Server / Pi'],
    ['updates', 'Updates'],
    ['trouble', 'Troubleshooting'],
  ] as const;
</script>

<div class="page">
  <h2 style="margin:0">Help</h2>
  <p class="lead muted">How to set up Ramp Radar — trackers, live data, and alerts. Links open in your browser.</p>
  <nav class="toc" aria-label="Help sections">
    {#each toc as [id, label]}
      <a href="#{id}">{label}</a>
    {/each}
  </nav>

  <section id="start" class="card">
    <h3 style="margin-top:0">Quick start</h3>
    <ol>
      <li>Open the <b>Trackers</b> tab and tap <b>+ New tracker</b>.</li>
      <li>Optionally set a <b>Label</b> (for example “Dad's Bonanza”). If you leave it blank, the aircraft identifier is used.</li>
      <li>In <b>Aircraft — tail number, Mode S hex, or callsign</b>, type something like <code>N800BS</code>, <code>A4F123</code>, or <code>DAL42</code>. Ramp Radar classifies it automatically and, for a US tail number, shows the matching Mode S hex. A tail blocked from public feeds can often still be followed if you enter its Mode S hex instead.</li>
      <li>Search <b>Airport</b> by ICAO, IATA, name or city (for example KBOS or Boston) and pick one.</li>
      <li>Set the <b>Alert radius</b> and <b>Unit</b> (<code>nm</code>, <code>mi</code>, or <code>km</code>). Any number is fine; the allowed range is about 0.05–500 nautical miles.</li>
      <li>Under <b>Alert when</b>, choose <b>enters radius</b>, <b>leaves radius</b>, <b>lands</b>, and/or <b>departs</b>.</li>
      <li>Optionally set an <b>Altitude ceiling (ft)</b> to ignore aircraft flying above that height, and <b>Quiet hours</b> (default 22:00–07:00, wrapping past midnight). Events in quiet hours are still written to the Log, but notifications are not sent.</li>
      <li>Under <b>Notify via</b>, pick channels for this tracker. New trackers start with <b>Desktop / browser</b> and <b>Sound</b>. Turn on ntfy, Email, Webhook, or SMS (Twilio) here after you have set them up in Settings.</li>
      <li>Pick a colour, leave <b>Enabled</b> checked, and tap <b>Save</b>. Use <b>Test alert</b> on a saved tracker to fire its selected channels.</li>
    </ol>
    <p>Click a card on the right to open its settings on the left (on a phone, the editor fills the screen). Card statuses:</p>
    <ul>
      <li><b>Not seen</b> — no recent position for this aircraft.</li>
      <li><b>Inbound</b> — airborne and heading toward the airport, still outside the radius.</li>
      <li><b>Within radius</b> — inside the alert radius around the airport.</li>
      <li><b>On ground</b> — reported on the ground at or near the airport.</li>
      <li><b>Outbound</b> — left the area or departed.</li>
      <li><b>Disabled</b> — this tracker is turned off.</li>
    </ul>
    <p>Cards also show distance, ETA, altitude, last seen, and which data source last answered. A red “data error” means every source failed for that poll.</p>
    <p>The <b>Map</b> tab shows the focused airport (the selected tracker, or the first one), its radius ring, tracked aircraft with heading and a short trail, and optional <b>Nearby traffic</b>. The <b>Log</b> tab lists enter / leave / landing / departure events; you can filter by tracker and <b>Export CSV</b>.</p>
    <p class="muted small">The header dot shows the data source currently in use. Under Settings you can change distance units, theme (System, Light, or Dark), and the poll interval (default 15 s; 15 s or more is polite for community APIs, down to 5 s for a local receiver).</p>
  </section>

  <section id="sources" class="card">
    <h3 style="margin-top:0">Data sources</h3>
    <p>Settings → <b>Data sources</b>. Sources are tried <b>top to bottom</b>; if one fails or is rate-limited, the next one is used. Use the up and down buttons beside each source to change the order, then <b>Save settings</b>. A failing source backs off automatically (it honours a <code>Retry-After</code> header, otherwise waits longer after each failure, up to 5 minutes).</p>
    <p>The default order is: local receiver (off until you enter a URL), then the three free community APIs (on), then OpenSky, ADS-B Exchange, and Simulated traffic (off).</p>

    <h4>Local receiver (readsb / dump1090 / tar1090)</h4>
    <p>Best when you have your own ADS-B radio. No internet, no rate limits, and it can see aircraft that public feeds filter out. Enable it, paste the <code>aircraft.json</code> URL, move it to the top, and save. Typical URLs:</p>
    <ul>
      <li><code>http://raspberrypi.local/tar1090/data/aircraft.json</code></li>
      <li><code>http://&lt;pi-address&gt;/tar1090/data/aircraft.json</code></li>
      <li><code>http://&lt;pi-address&gt;:8080/data/aircraft.json</code> (common dump1090 / readsb port)</li>
      <li><code>http://localhost/tar1090/data/aircraft.json</code> if the receiver is on the same machine as the server</li>
    </ul>
    <p>The URL must return readsb-style JSON (an <code>aircraft</code> or <code>ac</code> array). SBS/Beast TCP is not supported yet.</p>

    <h4>adsb.lol — free community API (ODbL)</h4>
    <p>No key. Data is licensed under the <a href="https://opendatacommons.org/licenses/odbl/" target="_blank" rel="noopener noreferrer">ODbL</a> — attribution is shown in Settings → About. Home: <a href="https://adsb.lol" target="_blank" rel="noopener noreferrer">adsb.lol</a>. A good default first internet source.</p>

    <h4>airplanes.live — free community API</h4>
    <p>No key. Intended for personal, non-commercial use at about 1 request/second (Ramp Radar already spaces calls ~1.1 s apart). Some cloud and VPN addresses get HTTP 403; failover then tries the next source. Terms: <a href="https://airplanes.live" target="_blank" rel="noopener noreferrer">airplanes.live</a>.</p>

    <h4>adsb.fi — free community API</h4>
    <p>No key. Personal, non-commercial use, also about 1 request/second. Terms: <a href="https://adsb.fi" target="_blank" rel="noopener noreferrer">adsb.fi</a>.</p>

    <h4>OpenSky Network (OAuth client credentials)</h4>
    <p>Free, but anonymous access is heavily limited. For a usable quota, create an account and an API client at <a href="https://opensky-network.org" target="_blank" rel="noopener noreferrer">opensky-network.org</a>, enable this source, paste the <b>OAuth client id</b>, and save the <b>OpenSky client secret</b> (write-only). OpenSky can look up a Mode S hex or an area; it has no callsign search. A US tail number is converted to hex first; a non-US registration is skipped and the next source is tried.</p>

    <h4>ADS-B Exchange (your own RapidAPI key)</h4>
    <p>Uses the RapidAPI endpoint (quota and price are set by them — we never ship a key). Subscribe at <a href="https://rapidapi.com/adsbx/api/adsbexchange-com1" target="_blank" rel="noopener noreferrer">ADS-B Exchange on RapidAPI</a>, enable this source, and save the <b>RapidAPI key</b>. See also <a href="https://www.adsbexchange.com" target="_blank" rel="noopener noreferrer">adsbexchange.com</a>.</p>

    <h4>Simulated traffic (demo / testing)</h4>
    <p>Built-in fake aircraft around an airport. Enable it in Settings, or start the server with <code>--mock</code> (that turns it on and puts it first). No internet required.</p>

    <p class="muted small">Please respect each service's rate limits and terms; they can change, and most community feeds are for personal use. Licence terms are also summarised in Settings → About &amp; data attribution.</p>
  </section>

  <section id="notify" class="card">
    <h3 style="margin-top:0">Notifications</h3>
    <p>Two switches have to be on for a channel to fire: the channel is ticked on the <b>tracker</b>, and (for ntfy, email, webhook, and SMS) it is <b>Enabled</b> in Settings. After filling in a channel, tap its <b>Send test</b> (desktop uses a button labelled <b>Test</b>). A test saves settings first, then sends a message. On a tracker, <b>Test alert</b> exercises that tracker's selected channels.</p>

    <h4>Desktop / browser and sound</h4>
    <ol>
      <li>In Settings → Notifications, leave <b>In-app sound</b> and/or <b>Desktop / browser notifications</b> checked (both are on by default).</li>
      <li>On the desktop app, alerts use the operating system's notification center.</li>
      <li>In a browser (server / Pi mode), tap <b>Allow browser notifications</b> and accept the permission prompt. This button only appears in server mode.</li>
      <li>Tap <b>Test</b> to confirm. Sound is a short in-app beep; it needs the Ramp Radar tab or window to be open.</li>
    </ol>

    <h4>ntfy.sh</h4>
    <ol>
      <li>Install the <a href="https://ntfy.sh" target="_blank" rel="noopener noreferrer">ntfy</a> app on your phone, or use the web app.</li>
      <li>Enable <b>ntfy.sh</b> in Settings. Leave <b>Server</b> as <code>https://ntfy.sh</code>, or put your self-hosted server URL there.</li>
      <li>Set <b>Topic</b> to a long, unguessable name. Anyone who knows a public topic can read it — don't use something like “my-plane”.</li>
      <li>In the ntfy app, subscribe to that same topic (and the same server, if you self-host).</li>
      <li>Optional: if your server requires auth, save an <b>Access token (optional)</b>. It is sent as a bearer token.</li>
      <li>Tap <b>Send test</b>, then confirm the message arrives on your phone.</li>
    </ol>

    <h4>Email (SMTP)</h4>
    <ol>
      <li>Enable <b>Email (SMTP)</b>.</li>
      <li>Fill in <b>Host</b>, <b>Port</b>, and <b>Security</b>: <code>STARTTLS</code> (typical on port 587, the default), <code>TLS</code> (typical on port 465), or <code>None</code>.</li>
      <li>Set <b>Username</b>, <b>From</b> (a plain address or <code>Ramp Radar &lt;me@example.com&gt;</code>), and <b>To</b>.</li>
      <li>Save the <b>SMTP password</b>. For Gmail, use an <a href="https://myaccount.google.com/apppasswords" target="_blank" rel="noopener noreferrer">App password</a> (2-Step Verification must be on), host <code>smtp.gmail.com</code>, port <code>587</code>, security <code>STARTTLS</code>, and your full Gmail address as the username and From address.</li>
      <li>Tap <b>Send test</b>.</li>
    </ol>

    <h4>Webhook</h4>
    <p>Enable <b>Webhook</b>, save the <b>Webhook URL</b> (it is stored as a secret), and tap <b>Send test</b>. Ramp Radar POSTs JSON:</p>
    <pre>{`{
  "title": "…",
  "message": "…",
  "text": "title: message",
  "source": "ramp-radar",
  "ts": "2026-11-14T19:00:00+00:00"
}`}</pre>
    <p class="muted small"><code>text</code> is <code>title: message</code>, handy for Slack incoming webhooks and similar. <code>ts</code> is UTC in RFC 3339 format.</p>

    <h4>SMS via Twilio (your own account)</h4>
    <ol>
      <li>Create an account at <a href="https://www.twilio.com/try-twilio" target="_blank" rel="noopener noreferrer">Twilio</a>. A trial includes some credit, but can only text numbers you have verified.</li>
      <li>Buy a phone number in the Twilio console. US numbers are typically about $1/month plus about a cent per text.</li>
      <li>Enable <b>SMS via Twilio</b> in Settings. Paste your <b>Account SID</b>. Set <b>From</b> to the Twilio number and <b>To</b> to your cell, both in E.164 form (for example <code>+15551230000</code>).</li>
      <li>Save the <b>Auth token</b> from the Twilio console, then tap <b>Send test</b>. Texts are sent as <code>title: message</code>, trimmed to 300 characters.</li>
      <li>For reliable delivery to US mobiles from a business number, carriers usually require <a href="https://www.twilio.com/docs/sms/a2p-10dlc" target="_blank" rel="noopener noreferrer">A2P 10DLC registration</a>, which takes a few days and has a small fee. If you only want alerts on your own phone, ntfy is free and does not need that.</li>
    </ol>
  </section>

  <section id="privacy" class="card">
    <h3 style="margin-top:0">Privacy &amp; security</h3>
    <ul>
      <li>Nothing is sent anywhere unless you configure a notification channel.</li>
      <li>Tokens and passwords are <b>write-only</b>: after you save them, the app shows only “stored”, never the value, and the API never returns them. On the desktop app they go in the OS keychain (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux). In server / Pi mode they are stored in a <code>secrets.json</code> file with permissions <code>0600</code> inside the data directory.</li>
      <li><b>Block list</b> (Settings → Privacy): one Mode S hex, registration, or callsign per line. Those aircraft cannot be tracked and never appear on the map.</li>
      <li>Aircraft in the FAA <b>LADD</b> (Limiting Aircraft Data Displayed) or <b>PIA</b> (Privacy ICAO Address) programmes are withheld or anonymised by public feeds on purpose. Ramp Radar does not try to bypass that. If a tail never shows up, try its Mode S hex, or use your own receiver.</li>
      <li>If you expose the server on your network, set an <b>access token</b> (<code>RR_ACCESS_TOKEN</code> or <code>--access-token</code>). Open <code>http://&lt;host&gt;:8080/?token=…</code> once; a cookie remembers it. The server warns at start-up if it is reachable off your computer and no token is set. Put it behind HTTPS if it leaves your home network.</li>
    </ul>
  </section>

  <section id="server" class="card">
    <h3 style="margin-top:0">Server / Pi mode</h3>
    <p>The same UI runs in any browser when you start the headless server (Raspberry Pi, a Linux box, or Docker). Installers and <code>ramp-radar-server-linux-arm64.tar.gz</code> / <code>x86_64</code> are on the <a href="https://github.com/Ethan-Poag/ramp-radar/releases/latest" target="_blank" rel="noopener noreferrer">latest GitHub release</a>.</p>
    <p>From a built binary:</p>
    <pre>./ramp-radar-server
# then open http://127.0.0.1:8080</pre>
    <p>From source: <code>cd web && npm ci && npm run build && cd ..</code>, then <code>cargo run --release -p ramp-radar-server</code>. Add <code>--mock</code> for simulated traffic.</p>
    <p>Useful options (flag or environment variable):</p>
    <ul>
      <li><code>--bind</code> / <code>RR_BIND</code> — default <code>127.0.0.1:8080</code>. Use <code>0.0.0.0:8080</code> to reach it on the LAN, and always set a token.</li>
      <li><code>--data-dir</code> / <code>RR_DATA_DIR</code> — database, secrets, and airport cache (default is your user data directory, <code>ramp-radar</code>).</li>
      <li><code>--access-token</code> / <code>RR_ACCESS_TOKEN</code> — required for a safe LAN install. Visit <code>http://&lt;pi-address&gt;:8080/?token=…</code> once.</li>
      <li><code>--mock</code>, <code>--mock-center</code> (airport ident, default KBOS), <code>--mock-scale</code> (time acceleration), <code>--offline</code> (don't download the airport list).</li>
    </ul>
    <p>A systemd unit is in the server tarball (<code>ramp-radar.service</code>). After installing the binary to <code>/usr/local/bin</code> and creating a <code>ramp-radar</code> system user with home <code>/var/lib/ramp-radar</code>:</p>
    <pre>sudo systemctl edit ramp-radar
# [Service]
# Environment=RR_ACCESS_TOKEN=&lt;long random token&gt;
sudo systemctl enable --now ramp-radar</pre>
    <p>Docker: <code>RR_ACCESS_TOKEN=change-me docker compose -f deploy/docker-compose.yml up -d --build</code> from the repo (port 8080, data in a volume). The headless build does not auto-update — install a newer server binary or image when you want one.</p>
  </section>

  <section id="updates" class="card">
    <h3 style="margin-top:0">Updates</h3>
    <p>The <b>desktop app</b> (Windows, macOS, Linux) checks GitHub for a new version when it starts. If one is available, a banner offers <b>Update now</b>. You can also open Settings → <b>Updates</b> and tap <b>Check for updates</b>. Updates are signed, so the app will refuse a package that was not built by this project.</p>
    <p>There is no auto-updater in server / Pi / browser mode — grab a new release and replace the binary or container. Settings and trackers stay in the data directory.</p>
    <p class="muted small">If you are still on a desktop version older than 0.1.2, install a current build once by hand; later versions will then update themselves.</p>
  </section>

  <section id="trouble" class="card">
    <h3 style="margin-top:0">Troubleshooting</h3>
    <ul>
      <li><b>Aircraft stays “Not seen.”</b> Confirm the identifier (a hint appears under the field). Public feeds hide LADD/PIA and some blocked tails — try the Mode S hex, or a local receiver. OpenSky cannot search by callsign. Check the header: “data source error” means every enabled source failed.</li>
      <li><b>HTTP 403 from a feed.</b> airplanes.live in particular may block cloud or VPN addresses. Leave several community sources enabled so failover can skip the blocked one.</li>
      <li><b>HTTP 429 / rate limited.</b> The source will back off (including any Retry-After the server sent). Raise the poll interval to 15 s or more, and avoid enabling more sources than you need at a very short interval.</li>
      <li><b>No alerts, but the card updates.</b> Is the tracker Enabled? Are the right “Alert when” boxes checked? Are you inside quiet hours (events still go to the Log)? Is the plane above the altitude ceiling? Is the channel ticked on the tracker and Enabled in Settings? Use <b>Test alert</b> and each channel's <b>Send test</b> / <b>Test</b>. In a browser, allow notification permission. For email/SMS, check host, ports, E.164 numbers, and that the secret is marked “stored.”</li>
      <li><b>Local receiver fails.</b> Open the <code>aircraft.json</code> URL in a regular browser from the same machine. Confirm the path (tar1090 vs dump1090-fa vs port 8080) and that Ramp Radar can reach that host.</li>
      <li><b>Windows warns on install.</b> Builds are not code-signed. Click More info, then Run anyway. On macOS, right-click the app and choose Open, or remove the quarantine flag as described in the README.</li>
      <li><b>Can't reach the server on the LAN.</b> Bind to <code>0.0.0.0:8080</code>, set <code>RR_ACCESS_TOKEN</code>, and open <code>http://&lt;host&gt;:8080/?token=…</code> once. Check the firewall on the Pi or PC.</li>
    </ul>
    <p class="muted small">More detail is in the <a href="https://github.com/Ethan-Poag/ramp-radar#readme" target="_blank" rel="noopener noreferrer">README</a>. Issues and ideas: <a href="https://github.com/Ethan-Poag/ramp-radar/issues" target="_blank" rel="noopener noreferrer">GitHub issues</a>.</p>
  </section>
</div>

<style>
  .page { padding: 1rem; max-width: 800px; margin: 0 auto; padding-bottom: 3rem; }
  .lead { margin: .35rem 0 .8rem; }
  .toc { position: sticky; top: 0; z-index: 4; display: flex; flex-wrap: wrap; gap: .4rem; padding: .55rem 0 .7rem; margin: 0 0 .4rem; background: var(--bg); }
  .toc a { display: inline-flex; align-items: center; min-height: 36px; padding: .25rem .7rem; border: 1px solid var(--line); border-radius: 999px; background: var(--panel); color: var(--text); text-decoration: none; font-size: .82rem; font-weight: 600; }
  .toc a:hover { border-color: var(--accent); color: var(--accent); }
  section { padding: 1rem 1.2rem; margin-bottom: 1rem; scroll-margin-top: 3.6rem; }
  h4 { font-size: .95rem; margin: 1.05rem 0 .35rem; }
  p, li { overflow-wrap: anywhere; }
  p { margin: .45rem 0; }
  ol, ul { margin: .35rem 0 .6rem; padding-left: 1.25rem; }
  li { margin: .28rem 0; }
  a { color: var(--accent); }
  code, pre { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .84em; }
  code { background: var(--panel2); border: 1px solid var(--line); border-radius: 4px; padding: .05rem .28rem; }
  pre { background: var(--panel2); border: 1px solid var(--line); border-radius: 8px; padding: .7rem .85rem; overflow-x: auto; white-space: pre-wrap; }
</style>
