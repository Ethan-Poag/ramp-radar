# Contributing

Thanks for helping! Issues and PRs are welcome.

## Layout

| Path | What |
|---|---|
| `crates/core` | all logic: `geo`, `nnumber`, `engine` (pure state machine), `providers/`, `notify`, `store`, `secrets`, `api` |
| `crates/server` | headless binary (clap) |
| `src-tauri` | desktop shell — separate Cargo workspace so the core builds without GTK/WebKit |
| `web` | Svelte 5 UI |
| `deploy` | Dockerfile, compose, systemd unit |

## Setup

* Rust stable, Node 20+.
* Desktop work on Linux: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libdbus-1-dev libssl-dev pkg-config`.
* `cd web && npm ci && npm run build` once (the server embeds `web/dist`; `cargo test` works without it).

## Before you open a PR

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd web && npm run check && npm run build)
cargo check --manifest-path src-tauri/Cargo.toml     # if you touched core APIs used by the desktop shell
```

## Guidelines

* **Pure logic gets tests.** Proximity rules live in `engine::evaluate` (no I/O) — add a test for every rule change.
* **Adding a data source:** implement `providers::Provider`, add a `ProviderConfig` variant in `settings.rs`, wire it in
  `Chain::from_settings`, add the TS type + a row in `SettingsView.svelte`. Unit-test the response parser with a
  captured sample; return `ProviderError::Unsupported` for query kinds the API can't do (so failover skips quietly).
  Never ship API keys; secrets are looked up by name from the `SecretStore`.
* **Adding a notification channel:** extend `ChannelKind`, `Notifier::send`, settings + UI. Errors must be
  sanitised — never include URLs with credentials or secret values in messages or logs.
* **Secrets:** never log, serialise into settings, or return from the API. Add new names to `secrets::KNOWN`.
* **Be polite to free APIs** (rate limits, caching, no polling faster than needed).
* Commit messages: imperative, short subject, explain *why* in the body.

## Releasing

Tag `vX.Y.Z` → `.github/workflows/release.yml` builds desktop installers (Windows, macOS arm64/x64, Linux) as a GitHub release and attaches `ramp-radar-server-linux-{x86_64,arm64}.tar.gz`.
