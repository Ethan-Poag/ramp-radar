# Ramp Radar — Design

Successor to the old Windows/Tkinter "Aircraft Proximity Monitor" (one aircraft, one airport,
Textbelt SMS, adsb.one). This app watches **many** aircraft ("trackers"), alerts through
several channels, shows a live map, and runs either as a desktop app or a headless LAN server.

## Architecture

```
                 ┌────────────────────── crates/core (library) ──────────────────────┐
 providers ────► │ poller ─► engine (pure state machine) ─► events ─► notifier fan-out │
 (readsb, lol,   │              │                              │                       │
  airplanes.live,│              ▼                              ▼                       │
  adsb.fi, OSky, │        live state (RAM)              SQLite store                   │
  ADSBx, mock)   │              │                                                      │
                 │        axum Router: REST + SSE + embedded Svelte UI                 │
                 └──────────────┬──────────────────────────────┬──────────────────────┘
                                │                              │
                  crates/server (bin: `ramp-radar-server`)   src-tauri (desktop)
                  0.0.0.0:8080, token auth, 0600 secrets file      starts the same Router on
                  Pi / Docker / systemd                            127.0.0.1:<random>, webview
                                                                   points at it; OS keychain,
                                                                   native notifications
```

One Rust core, one web UI (`web/`, Svelte 5 + TypeScript + Vite + MapLibre GL). The desktop
shell is deliberately thin: it hosts the *same* HTTP app in-process, so there is a single UI
code path and no Tauri-specific frontend code.

Layout:

```
crates/core/    geo, nnumber, airports, providers/, engine, notify, store, secrets, api, ui (rust-embed)
crates/server/  CLI wrapper (clap): --bind --data-dir --mock
src-tauri/      Tauri 2 shell (keyring secrets, tauri-plugin-notification)
web/            Svelte UI
deploy/         Dockerfile, systemd unit, Pi notes
.github/        CI + release workflows
```

## UI wireframe

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ ✈ Ramp Radar           [Trackers] [Map] [Log] [Settings]        ● adsb.lol ok │
├───────────────────────────────┬──────────────────────────────────────────────┤
│ SETTINGS (selected tracker)   │  [+ New tracker]                             │
│ Label  [Dad's Bonanza     ]   │ ┌──────────────────────────────────────────┐ │
│ Aircraft [N800BS ] (tail/hex/ │ │ ● N800BS  Bonanza   WITHIN RADIUS        │ │
│           callsign)           │ │ KBOS · 4.2 nm · ETA 2 min · 1,800 ft     │ │
│ Airport [KBOS Boston Logan ▾] │ │ last seen 3 s ago                        │ │
│ Radius  [10] [nm ▾]           │ └──────────────────────────────────────────┘ │
│ Alert on [x]enter [x]exit     │ ┌──────────────────────────────────────────┐ │
│          [x]landing           │ │ ● A4F123  INBOUND     PHNL 41 nm  ETA 9m │ │
│ Ceiling [    ] ft             │ └──────────────────────────────────────────┘ │
│ Quiet hours [22:00]–[07:00]   │ ┌──────────────────────────────────────────┐ │
│ Channels [x]ntfy [x]desktop   │ │ ○ DAL42   NOT SEEN                       │ │
│ Color [■]  [x] enabled        │ └──────────────────────────────────────────┘ │
│ [Save] [Test alert] [Delete]  │                                              │
└───────────────────────────────┴──────────────────────────────────────────────┘
 Phone: panes stack (cards first, editor in a slide-over), bottom tab bar.
```

Map tab: MapLibre + OSM raster tiles; airport marker, radius rings, tracked aircraft
(rotated arrow by track, tracker color, trail polyline), optional "all nearby traffic".

## Provider trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> Caps;            // which Query kinds natively supported
    fn min_interval(&self) -> Duration;        // rate limit, enforced by ProviderChain
    async fn fetch(&self, q: &Query) -> Result<Vec<Aircraft>, ProviderError>;
}
pub enum Query { Hex(String), Reg(String), Callsign(String), Point{lat,lon,radius_nm} }
```

`ProviderChain` holds the user's ordered list; each provider has health (consecutive failures,
back-off). Request goes to the first healthy provider; on error it fails over to the next.
Local receivers return the full `aircraft.json`, which the adapter filters locally.
US tails are converted to ICAO hex algorithmically so the exact `hex` query can be used.

## Data model (SQLite + JSON columns)

- `Tracker { id, label, target: Target{Reg|Hex|Callsign}, airport_ident, radius{value,unit},
  alert_enter, alert_exit, alert_landing, alert_takeoff, ceiling_ft?, quiet_hours?{start,end},
  channels[], color, enabled }`
- `Settings { providers[], poll_interval_s, units, theme, notifications{ntfy,email,webhook,twilio,
  sound,desktop}, blocklist[] }` — **no secret values**
- `Secrets` — name → value; file (0600) or OS keychain. API only reveals "is set".
- `events(id, tracker_id, ts, kind, airport, distance_nm, alt_ft, lat, lon, detail)`;
  kinds: `enter exit landing takeoff`. CSV export per tracker or all.
- Live `TrackerState { status, distance_nm, eta_min, alt_ft, gs_kt, track, lat, lon,
  last_seen, provider, trail[] }` kept in RAM, pushed over SSE.

Status: `not_seen | inbound | within | on_ground | outbound | disabled`.

## Engine

`evaluate(tracker, prev_state, observation, now) -> (state, events)` is a pure function, unit
tested. Hysteresis (exit at radius × 1.05) prevents flapping; altitude ceiling makes an aircraft
"inbound" until it is below the ceiling; quiet hours suppress *delivery*, not logging.

## Security / privacy

Secrets never serialized to API/logs (custom `Debug`, redacted errors). Server: optional access
token, warns when bound off-loopback without one. Block list prevents tracking/displaying listed
hex/regs. Nothing is published anywhere unless the user configures a channel. Note on FAA
LADD/PIA in README: those aircraft don't appear on public feeds by design — we don't try to evade.

## Milestones

1. **MVP (this commit series)**: core, server, mock + adsb.lol, CRUD, cards, map, ntfy + desktop events, SQLite, tests.
2. Email/webhook/Twilio hardened with integration tests, OpenSky/ADSBx live verification.
3. SBS/Beast TCP input; multi-user auth; i18n.
4. Signed installers, auto-update, mobile PWA polish.
