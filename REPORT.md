# Build report

Status of the first build of Aircraft Monitor (MVP), as of this commit series.

## What works (verified)

| Goal | Evidence |
|---|---|
| Server runs end-to-end, live data | Ran `aircraft-monitor-server` against **adsb.lol live**; 5 trackers (tail, hex, callsign) near KBOS/KPVD/PHNL showed real aircraft as *within / inbound / on ground* with distance, ETA, altitude; events logged to SQLite. |
| Map with live traffic near KBOS | `docs/screenshots/map.png` — MapLibre + OSM, radius rings, trails, heading-rotated aircraft, 29 nearby aircraft. |
| Provider failover | Observed for real: adsb.lol → HTTP 429 and airplanes.live → HTTP 403 from this network; the chain backed them off and served from adsb.fi. |
| Mock provider end-to-end | `--mock --mock-scale 60`: enter / exit / landing events fired and were persisted. |
| **Alert test passes** | `crates/core/tests/e2e.rs::proximity_alert_reaches_ntfy`: mock traffic → poll → engine → SQLite → **ntfy HTTP POST** received by a local fake ntfy server with correct topic, message and Bearer token. |
| Tests | `cargo test --workspace`: **44 unit + 6 end-to-end tests pass** (geo, proximity engine incl. hysteresis/ceiling/landing/stale, N-number ⇄ hex incl. exhaustive round-trip of all 915,399 US addresses and a real aircraft N707QS ⇄ A96FFE, airports, store, CSV escaping, secrets 0600, API CRUD/validation, token auth, DNS-rebinding guard, write-only secrets). `clippy -D warnings` and `fmt` clean. |
| **Desktop (Tauri 2) compiles and runs on Linux** | See below — `cargo check`, full debug **link**, and a smoke run under Xvfb. |
| CI config | `.github/workflows/ci.yml` (tests, clippy, web check/build, `cargo check` of the desktop crate on Linux/macOS/Windows) and `release.yml` (desktop installers for Windows, macOS arm64+x64, Linux; server tarballs for linux x86_64 + arm64; multi-arch Docker build). **Not executed** — no GitHub runners here; see "Unverified". |
| Screenshots | `docs/screenshots/`: trackers (light/dark), map (light/dark), log, settings, phone trackers/editor/map. Taken with headless Chrome via `web/scripts/screenshots.mjs`. |

### Desktop build on Linux — how it was verified

`apt` is unusable in this sandbox (the Debian mirrors return HTTP 500 through the proxy), so system packages
could not be installed normally. Instead I downloaded the needed `-dev` packages (170 packages incl.
`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`) directly from deb.debian.org over HTTPS, **verified each SHA256** against
the Packages index, and extracted them into a throwaway sysroot in `/tmp` (nothing installed system-wide), pointing
`pkg-config` at it. Results:

1. `cargo check` on `src-tauri` — clean, no warnings.
2. `cargo build` (debug) links a working `aircraft-monitor-desktop` binary.
3. Smoke run under Xvfb: the in-process core starts (downloaded 72,693 airports), serves on a random loopback port
   with `mode: "desktop"` and the native-notification hook registered; a foreign `Host` header gets 403; WebKit's
   web/network processes start and load the page. (For this run only I temporarily symlinked WebKit's hard-coded
   helper directory under `/usr/lib` to the sysroot, and removed it afterwards.)

Not done: producing installers (`cargo tauri build` bundling), and macOS/Windows builds — those are covered by CI only.

## How to run

```bash
cd web && npm ci && npm run build && cd ..
cargo run --release -p aircraft-monitor-server                 # live data, http://127.0.0.1:8080
cargo run --release -p aircraft-monitor-server -- --mock       # offline simulator
cd src-tauri && cargo tauri dev                                # desktop (needs GTK/WebKit dev packages)
```
More (Pi, Docker, systemd, env vars): `README.md`.

## Unverified / known gaps

* **CI workflows have never run.** Likely first-run fixes: runner labels (`macos-13`, `ubuntu-22.04-arm`), Tauri action
  inputs, unsigned-build warnings.
* **Dockerfile / docker-compose untested** (no Docker here). The arm64 server build is only configured in CI.
* **Email (SMTP), generic webhook, Twilio, OpenSky OAuth and ADS-B Exchange are implemented but not exercised against
  real services** (no credentials; I did not send anything to third parties). ntfy is covered by the e2e test.
  OpenSky/ADSBx/adsb.fi URL shapes and response parsers are unit-tested only against sample payloads.
* Native desktop notifications and OS-keychain storage are written but only compile-checked; no keychain/Secret Service
  daemon exists in the sandbox, and macOS/Windows paths are untested.
* Desktop notification "Test" in browser mode depends on the browser's Notification permission (button provided).
* No auth beyond one shared access token; no TLS (use a reverse proxy).
* Map tiles come from the public OSM server (fine for personal use; see README). Glyphs for map labels load from
  protomaps' public assets — the map works without them but labels would be missing offline.
* airplanes.live returned 403 from this network (similar to the old adsb.one problem) — failover handles it, but check
  their current terms/availability for your location.

## What's left / next milestones

1. **Run CI**, fix runner/bundle issues, publish a first signed release (Windows cert, Apple notarisation).
2. **Verify channels against real services** (SMTP, Twilio, OpenSky OAuth, ADSBx) and add integration tests with local fakes.
3. **Efficiency:** batch hex queries (`/v2/hex/a,b,c`) for many trackers; share one area query when several trackers use
   the same airport; SSE already fans out to all browsers from one poll.
4. **Local receiver depth:** SBS (port 30003) and Beast TCP inputs; direct readsb JSON on the same host.
5. **Better landing detection:** infer landing when an aircraft disappears below ~1,500 ft AGL near the airport
   (receivers often lose low aircraft); runway/ground-speed heuristics.
6. **Tracker features:** multiple airports per tracker, "any aircraft of type X", notification cooldown / digest,
   watch-lists import/export, per-tracker ntfy topics.
7. **UI:** selected-aircraft detail panel with altitude/speed sparkline, tile-provider setting, PWA/installable web app,
   i18n, keyboard shortcuts, offline vector basemap.
8. **Security hardening:** per-user accounts or OIDC for server mode, built-in TLS option, rate-limit the API.
9. **Tauri extras:** tray icon + run-in-background, autostart, auto-update.
