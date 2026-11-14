//! Ramp Radar desktop shell.
//!
//! Starts the shared Rust core (same axum router as the headless server) on a random
//! loopback port inside this process, then opens a webview pointing at it. Native
//! notifications and OS-keychain secrets are plugged into the core here.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ramp_radar_core::{
    api,
    app::{App, AppOptions},
    secrets::SecretStore,
    store::Store,
};
use std::sync::Arc;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;

const KEYRING_SERVICE: &str = "ramp-radar";

/// Secrets in the OS keychain (Keychain on macOS, Credential Manager on Windows,
/// Secret Service on Linux). Values never touch disk in our own files.
struct KeychainSecrets;

impl SecretStore for KeychainSecrets {
    fn get(&self, name: &str) -> Option<String> {
        keyring::Entry::new(KEYRING_SERVICE, name).ok()?.get_password().ok()
    }
    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(KEYRING_SERVICE, name)
            .and_then(|e| e.set_password(value))
            .map_err(|e| e.to_string())
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        match keyring::Entry::new(KEYRING_SERVICE, name).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let handle = app.handle().clone();

            // macOS needs an explicit permission request; harmless elsewhere.
            let _ = app.notification().request_permission();

            let port = tauri::async_runtime::block_on(async move {
                let store = Store::open(&data_dir.join("monitor.db")).expect("open database");
                let core = App::new(
                    AppOptions {
                        data_dir,
                        mode: "desktop",
                        access_token: None,
                        mock_center: (42.3643, -71.0052),
                        mock_scale: 1.0,
                        download_airports: true,
                    },
                    store,
                    Arc::new(KeychainSecrets),
                )
                .await;

                let h = handle.clone();
                core.notifier.set_desktop_hook(Arc::new(move |title, body| {
                    let _ = h.notification().builder().title(title).body(body).show();
                }));
                core.spawn_poller();

                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind loopback");
                let port = listener.local_addr().expect("addr").port();
                tauri::async_runtime::spawn(async move {
                    let _ = axum::serve(listener, api::router(core)).await;
                });
                port
            });

            let url = format!("http://127.0.0.1:{port}/");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse().expect("url")))
                .title("Ramp Radar")
                .inner_size(1200.0, 800.0)
                .min_inner_size(380.0, 600.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Ramp Radar");
}
