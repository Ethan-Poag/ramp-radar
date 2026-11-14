<script lang="ts">
  let q = $state('');

  const toc = [
    ['start', 'Get started'],
    ['phone', 'Phone alerts'],
    ['email', 'Email alerts'],
    ['missing', "Plane won't show up"],
    ['airports', 'More than one airport'],
    ['notseen', "Why 'Not seen'?"],
    ['antenna', 'Your own antenna'],
    ['frontdesk', 'Front desk / Pi'],
    ['updates', 'Updating'],
  ] as const;

  function visible(keywords: string) {
    const n = q.trim().toLowerCase();
    if (!n) return true;
    const hay = keywords.toLowerCase();
    return n.split(/\s+/).every((w) => hay.includes(w));
  }

  const anyVisible = $derived(
    visible('get started first tracker tail number airport radius card status inbound on ground map log alert') ||
      visible('phone text alert ntfy inbound sms twilio notification') ||
      visible('email alerts smtp gmail') ||
      visible("track plane won't show up missing hidden mode s hex tail blocked ladd") ||
      visible('watch more than one airport several locations') ||
      visible("why is my plane not seen data error") ||
      visible('using your own antenna receiver radio optional dump1090 tar1090') ||
      visible('running shared screen raspberry pi front desk lobby server access token docker install.sh curl linux') ||
      visible('updating the app update check for updates banner desktop install.sh linux server'),
  );
</script>

<div class="page">
  <h2 style="margin:0">Help</h2>
  <p class="lead muted">Plain-language setup for the front desk and ops. Links open in your browser.</p>
  <label class="search-wrap">
    <span class="visually-hidden">Search help</span>
    <input type="search" bind:value={q} placeholder="Search help…" aria-label="Search help" />
  </label>
  <nav class="toc" aria-label="Help sections">
    {#each toc as [id, label]}
      <a href="#{id}">{label}</a>
    {/each}
  </nav>
  {#if !anyVisible}
    <p class="muted">No matching topics. Try “phone”, “email”, or “not seen”.</p>
  {/if}

  {#if visible('get started first tracker tail number airport radius card status inbound on ground map log alert quiet hours label')}
  <section id="start" class="card">
    <h3 style="margin-top:0">Get started in 5 minutes</h3>
    <p>This walks you through watching one based customer and getting a ping when they come in. You can add more planes afterward the same way.</p>

    <h4>1. Add your first tracker</h4>
    <ol>
      <li>Open the <b>Trackers</b> tab and tap <b>+ New tracker</b>.</li>
      <li>In <b>Label</b>, type a name you will recognize, such as “Smith Bonanza”. If you leave this blank, the tail number is used.</li>
      <li>In <b>Aircraft — tail number, Mode S hex, or callsign</b>, type the tail number (the registration painted on the plane, like <code>N800BS</code>). A short note should appear under the box confirming it was recognized. You can also type a flight name used on the radio (a callsign, like <code>DAL42</code>), or a Mode S code (a 6-character ID every aircraft transmits, like <code>A4F123</code>).</li>
      <li>In <b>Airport</b>, type your airport name, city, or code (for example Boston or KBOS) and pick it from the list.</li>
      <li>Set <b>Alert radius</b> to how far out you want a heads-up — 10 nautical miles is a solid start. Change <b>Unit</b> if you prefer miles or kilometres.</li>
      <li>Under <b>Alert when</b>, leave <b>enters radius</b> and <b>lands</b> checked if you want to know when they are inbound and when they have arrived. Add <b>leaves radius</b> or <b>departs</b> if you also want departures.</li>
      <li>Leave <b>Enabled</b> checked, then tap <b>Save</b>.</li>
    </ol>
    <p>Optional: <b>Quiet hours</b> still records visits in the Log but will not send alerts (it starts at 22:00–07:00 if you turn it on). <b>Altitude ceiling (ft)</b> ignores planes flying above that height, which is handy if you only care about arrivals, not overflights.</p>

    <h4>2. Get the alert on your phone</h4>
    <p>The easiest free option is ntfy, a small app that shows a message on your phone. Details are in <a href="#phone">Get a text/phone alert when a plane is inbound</a>. After you set it up, open this tracker again, tick <b>ntfy</b> under <b>Notify via</b>, and tap <b>Save</b>.</p>
    <p>Until then, alerts can also pop up on this computer if <b>Desktop / browser</b> and <b>Sound</b> are ticked (they are on for new trackers by default).</p>

    <h4>3. Read the cards</h4>
    <p>Each plane is a card on the right. Tap a card to edit it (on a phone, the editor fills the screen). You can pick a colour so planes are easy to tell apart on the map. The status on the card means:</p>
    <ul>
      <li><b>Not seen</b> — no recent position for this aircraft.</li>
      <li><b>Inbound</b> — in the air, heading toward your airport, but still outside the radius.</li>
      <li><b>Within radius</b> — inside the area you chose around the airport.</li>
      <li><b>On ground</b> — reported on the ground at or near the airport.</li>
      <li><b>Outbound</b> — left the area or departed.</li>
      <li><b>Disabled</b> — this tracker is turned off, so it will not alert.</li>
    </ul>
    <p>Cards also show distance, time of arrival estimate, altitude, and when the plane was last seen. The <b>Map</b> tab shows your airport, the radius ring, and the planes you are watching. The <b>Log</b> tab is a visit history; you can filter it and tap <b>Export CSV</b> to save a spreadsheet.</p>
    <p class="muted small">Ramp Radar already looks up planes on free public flight data. You do not have to set that up to get started. Change units, Light/Dark theme, or other options anytime under <b>Settings</b>.</p>
  </section>
  {/if}

  {#if visible('phone text alert ntfy inbound sms twilio notification send test topic')}
  <section id="phone" class="card">
    <h3 style="margin-top:0">Get a text/phone alert when a plane is inbound</h3>
    <p><b>ntfy</b> is a free app for your phone. We recommend it. It takes a couple of minutes and does not need a paid account. (Carrier text messages are a paid option further down.)</p>

    <h4>Set up ntfy (free)</h4>
    <ol>
      <li>Install the <a href="https://ntfy.sh" target="_blank" rel="noopener noreferrer">ntfy</a> app on your phone, or open it in a phone browser.</li>
      <li>In Ramp Radar, open <b>Settings</b> and find <b>ntfy.sh</b>.</li>
      <li>Tick <b>Enabled</b>.</li>
      <li>Leave <b>Server</b> as <code>https://ntfy.sh</code> unless your IT person gave you a different address.</li>
      <li>In <b>Topic</b>, type a long, hard-to-guess name — treat it like a password. Anyone who can guess it could see your alerts, so do not use something like “fbo-alerts”.</li>
      <li>In the ntfy app, subscribe to that exact same topic name.</li>
      <li>Tap <b>Send test</b>. You should get a test message on your phone within a few seconds.</li>
      <li>Open each tracker that should ping your phone, tick <b>ntfy</b> under <b>Notify via</b>, and tap <b>Save</b>. Use <b>Test alert</b> on the tracker to confirm that plane is wired up.</li>
    </ol>
    <p>Success looks like: the test message arrives on your phone, and later a real alert arrives when that plane enters the radius or lands (depending on what you checked under <b>Alert when</b>).</p>

    <h4>Or use carrier text messages (paid)</h4>
    <p><b>SMS via Twilio</b> sends a normal text message. You need your own Twilio account. A U.S. number is typically about $1 per month plus about a cent per text.</p>
    <ol>
      <li>Create an account at <a href="https://www.twilio.com/try-twilio" target="_blank" rel="noopener noreferrer">Twilio</a> and buy a phone number. A trial account can only text numbers you have verified in Twilio.</li>
      <li>In <b>Settings</b>, tick <b>Enabled</b> under <b>SMS via Twilio (your own account)</b>.</li>
      <li>Paste your <b>Account SID</b> from the Twilio website.</li>
      <li>Set <b>From</b> to your Twilio number and <b>To</b> to the cell phone that should get the texts. Start each number with a plus and the country code, for example <code>+15551230000</code>.</li>
      <li>Paste the <b>Auth token</b> from Twilio and save it, then tap <b>Send test</b>.</li>
      <li>On each tracker, tick <b>SMS (Twilio)</b> under <b>Notify via</b> and tap <b>Save</b>.</li>
    </ol>
    <p>Success looks like: a test text arrives from your Twilio number. If you only want alerts on your own staff phones, ntfy is simpler and free.</p>
    <p class="muted small">U.S. carriers often require extra business-text registration before delivery is reliable, which takes a few days and has a small fee. Your Twilio dashboard explains that process if you go this route.</p>

    <details class="adv">
      <summary>Advanced / for IT</summary>
      <ul>
        <li>Optional ntfy <b>Access token (optional)</b> is sent as a bearer token, for a password-protected or self-hosted ntfy server. Put your self-hosted URL in <b>Server</b>.</li>
        <li>Twilio sends the body as <code>title: message</code>, trimmed to 300 characters. Numbers must include the country code.</li>
        <li>A channel only fires if it is ticked on the tracker <em>and</em> <b>Enabled</b> in Settings. <b>Send test</b> saves Settings first. In-app <b>Sound</b> and <b>Desktop / browser notifications</b> are separate; in a web browser tap <b>Allow browser notifications</b> if you see that button.</li>
        <li>You can also enable <b>Webhook</b> in Settings, save a <b>Webhook URL</b>, and tick <b>Webhook</b> on the tracker. Ramp Radar POSTs JSON: <code>title</code>, <code>message</code>, <code>text</code> (that is <code>title: message</code>), <code>source</code> (<code>ramp-radar</code>), and <code>ts</code> (UTC, RFC 3339).</li>
      </ul>
    </details>
  </section>
  {/if}

  {#if visible('email alerts smtp gmail mail send test')}
  <section id="email" class="card">
    <h3 style="margin-top:0">Get email alerts</h3>
    <p>Ramp Radar can email you through the mailbox you already use (Gmail, Microsoft 365, or your FBO’s mail server). There is no Ramp Radar email fee; your mail provider’s usual rules apply.</p>
    <ol>
      <li>Open <b>Settings</b> and tick <b>Enabled</b> under <b>Email (SMTP)</b>.</li>
      <li>Fill in <b>Host</b> (for Gmail: <code>smtp.gmail.com</code>), leave <b>Port</b> at <code>587</code> and <b>Security</b> on <b>STARTTLS</b> unless your IT person gives you different values.</li>
      <li>Set <b>Username</b> to the mailbox login (for Gmail, the full address).</li>
      <li>Set <b>From</b> to the address you are sending from (plain, or like <code>Ramp Radar &lt;ops@example.com&gt;</code>) and <b>To</b> to the inbox that should receive alerts.</li>
      <li>Save the <b>SMTP password</b>. For Gmail, do not use your normal password — create an <a href="https://myaccount.google.com/apppasswords" target="_blank" rel="noopener noreferrer">App password</a> (2-Step Verification must be on) and paste that.</li>
      <li>Tap <b>Send test</b>, then check the inbox (and spam) for the test message.</li>
      <li>On each tracker that should send email, tick <b>Email</b> under <b>Notify via</b> and tap <b>Save</b>.</li>
    </ol>
    <p>Success looks like: the test email arrives, and later a real email shows up when that plane meets the conditions you checked under <b>Alert when</b>.</p>
    <details class="adv">
      <summary>Advanced / for IT</summary>
      <ul>
        <li><b>Security</b> options are <b>STARTTLS</b> (typical on port 587), <b>TLS</b> (typical on port 465), and <b>None</b>. A password is required when <b>Username</b> is filled in.</li>
        <li>The email subject is the alert title; the body is the alert text. <b>Send test</b> saves Settings first.</li>
      </ul>
    </details>
  </section>
  {/if}

  {#if visible("track plane won't show up missing hidden mode s hex tail blocked ladd pia privacy")}
  <section id="missing" class="card">
    <h3 style="margin-top:0">Track a plane that won't show up</h3>
    <p>If a tail number never appears, it is often hidden from public flight websites on purpose. Some owners enroll in FAA privacy programs that keep their plane off those sites. Ramp Radar does not try to get around that.</p>
    <ol>
      <li>Double-check the tail number in the tracker and look at the short confirmation note under the box.</li>
      <li>Ask the owner for the plane’s <b>Mode S code</b> (a 6-character ID it transmits, like <code>A4F123</code>). Put that in the <b>Aircraft</b> field instead of the tail number — sometimes that still works when the tail is blocked from public lists.</li>
      <li>If you need to see planes that public sites hide, use <a href="#antenna">your own antenna</a>. A radio on the field sees what is actually in the air around you.</li>
    </ol>
    <p>You can also hide a plane from Ramp Radar entirely: in <b>Settings</b> → <b>Privacy</b>, add it to the <b>Block list</b> (one tail number, Mode S code, or callsign per line). Blocked aircraft cannot be tracked and never appear on the map.</p>
    <details class="adv">
      <summary>Advanced / for IT</summary>
      <ul>
        <li>The relevant FAA programs are LADD (Limiting Aircraft Data Displayed) and PIA (Privacy ICAO Address). Public feeds withhold or anonymise those aircraft by design.</li>
        <li>The <b>OpenSky Network</b> source cannot search by callsign. A U.S. tail number is converted to a Mode S code first; a non-U.S. registration is skipped and the next source is tried. Optional OAuth client id + <b>OpenSky client secret</b> improve the quota; anonymous access is very limited.</li>
        <li>ADS-B Exchange needs your own RapidAPI key, entered as <b>RapidAPI key</b> in Settings. Community sources (adsb.lol, airplanes.live, adsb.fi) need no key and are for personal, non-commercial use; see Settings → About &amp; data attribution.</li>
      </ul>
    </details>
  </section>
  {/if}

  {#if visible('watch more than one airport several locations trackers map')}
  <section id="airports" class="card">
    <h3 style="margin-top:0">Watch more than one airport</h3>
    <p>Each tracker is one plane plus one airport. To cover a second field, add another tracker — even for the same plane — and pick the other airport.</p>
    <ol>
      <li>On <b>Trackers</b>, tap <b>+ New tracker</b>.</li>
      <li>Enter the same or a different aircraft, then search and select the other <b>Airport</b>.</li>
      <li>Set the radius and alerts, then tap <b>Save</b>.</li>
    </ol>
    <p>You can have as many trackers as you need (based customers, transients you care about, a second campus). The map follows the tracker you have selected, or the first one if none is selected. Give each tracker a clear <b>Label</b> and colour so the front desk can tell them apart.</p>
  </section>
  {/if}

  {#if visible("why is my plane not seen data error 403 429 feed source quiet hours ceiling alerts")}
  <section id="notseen" class="card">
    <h3 style="margin-top:0">Why is my plane “Not seen”?</h3>
    <p>That status means Ramp Radar has no recent position for this aircraft. Common fixes:</p>
    <ul>
      <li>The plane may still be far away, on the ground at another airport, or powered off. “Not seen” is normal until it starts transmitting.</li>
      <li>Check the tail number (or try the Mode S code — see <a href="#missing">Track a plane that won't show up</a>).</li>
      <li>Look at the top of the app. If it says <b>data source error</b>, the live-data services Ramp Radar uses are all failing right now. Wait a minute, or check your internet connection.</li>
      <li>If you <em>do</em> see the plane on the card but get no alert, check that the tracker is <b>Enabled</b>, the right <b>Alert when</b> boxes are ticked, you are not inside <b>Quiet hours</b> (visits still go to the Log), the plane is not above the <b>Altitude ceiling</b>, and the alert method is ticked on the tracker and <b>Enabled</b> in Settings. Use <b>Test alert</b> and each <b>Send test</b> button.</li>
    </ul>
    <details class="adv">
      <summary>Advanced / for IT</summary>
      <ul>
        <li>Settings → <b>Data sources</b> are tried top to bottom; use the up and down buttons to change order, then <b>Save settings</b>. A failing source backs off automatically (it honours a Retry-After header, otherwise waits longer after each failure, up to 5 minutes).</li>
        <li>Default enabled internet sources are adsb.lol, airplanes.live, and adsb.fi. airplanes.live may return HTTP 403 for some cloud or VPN addresses; leave several sources on so the next one can take over. HTTP 429 means rate-limited — raise <b>Poll interval (s)</b> (default 15; 15 or more is polite for community APIs, down to 5 for a local receiver).</li>
        <li>OpenSky cannot search by callsign. A local receiver is preferred when you have one; see the antenna section.</li>
      </ul>
    </details>
  </section>
  {/if}

  {#if visible('using your own antenna receiver radio optional dump1090 tar1090 readsb aircraft.json local')}
  <section id="antenna" class="card">
    <h3 style="margin-top:0">Using your own antenna (optional, for best results)</h3>
    <p>If you already have an ADS-B radio on the field (a receiver that listens to the signals planes broadcast), Ramp Radar can use it. This is optional. Benefits: it works even when internet flight sites are spotty, and it can see planes that public websites hide.</p>
    <ol>
      <li>Open <b>Settings</b> → <b>Data sources</b>.</li>
      <li>Tick <b>Local receiver (readsb / dump1090 / tar1090)</b>.</li>
      <li>Paste the address of your receiver’s aircraft list (your installer or IT person usually has this) and move this source to the top with the up button.</li>
      <li>Tap <b>Save settings</b>. The header at the top should show <b>local</b> once it is working.</li>
    </ol>
    <p>Success looks like: tracked planes update smoothly, and the header dot stays green with the word <b>local</b>.</p>
    <details class="adv">
      <summary>Advanced / for IT</summary>
      <ul>
        <li>The URL must return readsb-style JSON (an <code>aircraft</code> or <code>ac</code> array). Typical addresses: <code>http://raspberrypi.local/tar1090/data/aircraft.json</code>, <code>http://&lt;pi-address&gt;/tar1090/data/aircraft.json</code>, <code>http://&lt;pi-address&gt;:8080/data/aircraft.json</code> (common dump1090 / readsb port), or <code>http://localhost/tar1090/data/aircraft.json</code> if the receiver is on the same machine. SBS/Beast TCP is not supported yet.</li>
        <li>A local source can be polled as often as every 5 seconds. It is first in the default list but stays off until a URL is entered.</li>
      </ul>
    </details>
  </section>
  {/if}

  {#if visible('running shared screen raspberry pi front desk lobby server access token docker install.sh curl linux')}
  <section id="frontdesk" class="card">
    <h3 style="margin-top:0">Running it on a shared screen / Raspberry Pi for the front desk</h3>
    <p>You can leave Ramp Radar open on a lobby or operations computer, or run it on a small always-on box (a Raspberry Pi) and open it in a browser on any screen, including a phone.</p>
    <ol>
      <li>On a front-desk PC, install the desktop app and leave it running, or open the browser address your IT person gives you.</li>
      <li>If the page asks for an access token, that is a password for this app. Enter it once; the browser should remember it.</li>
      <li>Use the same Trackers, Map, and Log screens described above. Phone layouts work in a phone browser too.</li>
    </ol>
    <p>If this is on your office network, ask IT to set that access token so visitors on the Wi-Fi cannot change your trackers. Your settings and history stay on that computer or Pi; nothing is published unless you turn on alerts.</p>
    <p>On Linux or a Raspberry Pi, someone with admin access can install it with one command — desktop app, or add <code>--server</code> for the always-on browser version (see Advanced).</p>
    <details class="adv">
      <summary>Advanced / for IT</summary>
      <ul>
        <li>Desktop (Debian/Ubuntu .deb, Fedora/RHEL .rpm, otherwise AppImage): <code>curl -fsSL https://raw.githubusercontent.com/Ethan-Poag/ramp-radar/main/install.sh | sh</code></li>
        <li>Headless server + systemd, then open <code>http://&lt;host&gt;:8080</code>: <code>curl -fsSL https://raw.githubusercontent.com/Ethan-Poag/ramp-radar/main/install.sh | sh -s -- --server</code></li>
        <li>Safer two-step: download <a href="https://raw.githubusercontent.com/Ethan-Poag/ramp-radar/main/install.sh" target="_blank" rel="noopener noreferrer">install.sh</a>, read it, then run <code>sh install.sh</code> or <code>sh install.sh --server</code>.</li>
        <li>Headless binaries are also on the <a href="https://github.com/Ethan-Poag/ramp-radar/releases/latest" target="_blank" rel="noopener noreferrer">latest GitHub release</a> (<code>ramp-radar-server-linux-arm64.tar.gz</code> / <code>x86_64</code>). From source: <code>cd web && npm ci && npm run build && cd ..</code>, then <code>cargo run --release -p ramp-radar-server</code>. Add <code>--mock</code> for simulated traffic.</li>
        <li>Flags or env vars: <code>--bind</code> / <code>RR_BIND</code> (default <code>127.0.0.1:8080</code>; use <code>0.0.0.0:8080</code> on a LAN and always set a token), <code>--data-dir</code> / <code>RR_DATA_DIR</code>, <code>--access-token</code> / <code>RR_ACCESS_TOKEN</code>, <code>--mock</code>, <code>--mock-center</code>, <code>--mock-scale</code>, <code>--offline</code>. Visit <code>http://&lt;host&gt;:8080/?token=…</code> once; a cookie is stored. The server warns if it is reachable off-loopback with no token.</li>
        <li>systemd unit: <code>ramp-radar.service</code> in the tarball. Install the binary to <code>/usr/local/bin</code>, create user <code>ramp-radar</code> with home <code>/var/lib/ramp-radar</code>, then <code>sudo systemctl edit ramp-radar</code> to set <code>Environment=RR_ACCESS_TOKEN=…</code> and <code>sudo systemctl enable --now ramp-radar</code>.</li>
        <li>Docker: <code>RR_ACCESS_TOKEN=change-me docker compose -f deploy/docker-compose.yml up -d --build</code>. The headless build does not auto-update.</li>
        <li>Secrets (tokens, passwords) are write-only — the app shows “stored”, never the value. Desktop uses the OS keychain; server mode uses <code>secrets.json</code> with permissions <code>0600</code> in the data directory.</li>
      </ul>
    </details>
  </section>
  {/if}

  {#if visible('updating the app update check for updates banner desktop install.sh linux server')}
  <section id="updates" class="card">
    <h3 style="margin-top:0">Updating the app</h3>
    <p>On a Windows, Mac, or Linux desktop, Ramp Radar checks for a new version when it starts. If one is ready, a banner offers <b>Update now</b>. You can also open <b>Settings</b> → <b>Updates</b> and tap <b>Check for updates</b>.</p>
    <p>Success looks like: the banner appears for a new version, you tap <b>Update now</b>, and the app restarts on the new version. Your trackers and settings are kept.</p>
    <p>If you open Ramp Radar in a browser on a Pi or server, it does not update itself. On Linux, IT can install or refresh the latest copy with the same one-line command in the front-desk / Pi topic (or the README). Settings stay in the data folder on that machine.</p>
    <p class="muted small">If this desktop copy is older than version 0.1.2, install a current build once by hand from the <a href="https://github.com/Ethan-Poag/ramp-radar/releases/latest" target="_blank" rel="noopener noreferrer">releases page</a>. Later versions will then update themselves.</p>
  </section>
  {/if}
</div>

<style>
  .page { padding: 1rem; max-width: 800px; margin: 0 auto; padding-bottom: 3rem; }
  .lead { margin: .35rem 0 .7rem; }
  .search-wrap { display: block; margin: 0 0 .55rem; }
  .search-wrap input { width: 100%; }
  .visually-hidden { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  .toc { position: sticky; top: 0; z-index: 4; display: flex; flex-wrap: wrap; gap: .4rem; padding: .55rem 0 .7rem; margin: 0 0 .4rem; background: var(--bg); }
  .toc a { display: inline-flex; align-items: center; min-height: 36px; padding: .25rem .7rem; border: 1px solid var(--line); border-radius: 999px; background: var(--panel); color: var(--text); text-decoration: none; font-size: .82rem; font-weight: 600; }
  .toc a:hover { border-color: var(--accent); color: var(--accent); }
  section { padding: 1rem 1.2rem; margin-bottom: 1rem; scroll-margin-top: 3.6rem; }
  section h3 { font-size: 1.15rem; text-transform: none; letter-spacing: 0; color: var(--text); font-weight: 700; }
  h4 { font-size: .95rem; margin: 1.05rem 0 .35rem; }
  p, li { overflow-wrap: anywhere; }
  p { margin: .45rem 0; }
  ol, ul { margin: .35rem 0 .6rem; padding-left: 1.25rem; }
  li { margin: .28rem 0; }
  a { color: var(--accent); }
  code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .84em; background: var(--panel2); border: 1px solid var(--line); border-radius: 4px; padding: .05rem .28rem; }
  .adv { margin-top: 1rem; border-top: 1px solid var(--line); padding-top: .6rem; }
  .adv summary { cursor: pointer; color: var(--muted); font-weight: 600; font-size: .88rem; min-height: 36px; display: flex; align-items: center; list-style-position: outside; }
  .adv summary:hover { color: var(--accent); }
  .adv ul { margin-top: .4rem; }
</style>
