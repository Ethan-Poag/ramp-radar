//! Ramp Radar desktop shell.
//!
//! Starts the shared Rust core (same axum router as the headless server) on a random
//! loopback port inside this process, then opens a webview pointing at it. Native
//! notifications and OS-keychain secrets are plugged into the core here.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use ramp_radar_core::{
    api,
    app::{App, AppOptions},
    secrets::SecretStore,
    store::Store,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;
use tower::ServiceExt;

const KEYRING_SERVICE: &str = "ramp-radar";

/// Secrets in the OS keychain (Keychain on macOS, Credential Manager on Windows,
/// Secret Service on Linux). Values never touch disk in our own files.
struct KeychainSecrets;

impl SecretStore for KeychainSecrets {
    fn get(&self, name: &str) -> Option<String> {
        keyring::Entry::new(KEYRING_SERVICE, name)
            .ok()?
            .get_password()
            .ok()
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
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
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

                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("bind loopback");
                let port = listener.local_addr().expect("addr").port();

                // The UI is served over loopback HTTP, so the desktop shell exposes
                // updater endpoints on the same server instead of relying on Tauri IPC.
                let updater = desktop_updater_router(handle.clone());
                let core_router = api::router(core);
                tauri::async_runtime::spawn(async move {
                    let svc =
                        Router::new().fallback_service(tower::service_fn(move |req: Request| {
                            let updater = updater.clone();
                            let core_router = core_router.clone();
                            async move {
                                if req.uri().path().starts_with("/api/desktop/") {
                                    updater.oneshot(req).await
                                } else {
                                    core_router.oneshot(req).await
                                }
                            }
                        }));
                    let _ = axum::serve(listener, svc).await;
                });
                port
            });

            let url = format!("http://127.0.0.1:{port}/");
            let opener = app.handle().clone();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse().expect("url")))
                .title("Ramp Radar")
                .inner_size(1200.0, 800.0)
                .min_inner_size(380.0, 600.0)
                .on_navigation(move |dest| {
                    let host = dest.host_str().unwrap_or("");
                    let loopback = matches!(host, "127.0.0.1" | "localhost" | "::1");
                    if matches!(dest.scheme(), "http" | "https") && loopback {
                        return true;
                    }
                    if matches!(dest.scheme(), "about" | "data" | "blob" | "devtools") {
                        return true;
                    }
                    let _ = opener.opener().open_url(dest.as_str(), None::<&str>);
                    false
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Ramp Radar");
}

fn desktop_updater_router(handle: tauri::AppHandle) -> Router {
    Router::new()
        .route(
            "/api/desktop/update",
            get({
                let handle = handle.clone();
                move || {
                    let handle = handle.clone();
                    async move { check_update(handle).await }
                }
            }),
        )
        .route(
            "/api/desktop/update/install",
            post({
                let handle = handle.clone();
                move || {
                    let handle = handle.clone();
                    async move { install_update(handle).await }
                }
            }),
        )
        .route(
            "/api/desktop/open",
            post({
                let handle = handle.clone();
                move |Json(body): Json<Value>| {
                    let handle = handle.clone();
                    async move { open_external(handle, body).await }
                }
            }),
        )
        .layer(middleware::from_fn(loopback_only))
}

async fn loopback_only(req: Request, next: Next) -> axum::response::Response {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let name = host.rsplit_once(':').map(|x| x.0).unwrap_or(host);
    if !matches!(name, "127.0.0.1" | "localhost" | "[::1]") {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(req).await
}

fn api_err(code: StatusCode, msg: impl Into<String>) -> (StatusCode, Json<Value>) {
    (code, Json(json!({ "error": msg.into() })))
}

fn parse_http_url(raw: &str) -> Result<String, (StatusCode, Json<Value>)> {
    let raw = raw.trim();
    let scheme = raw
        .split_once("://")
        .map(|(s, _)| s)
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https")
        || raw.chars().any(|c| c.is_ascii_whitespace() || c == '\0')
    {
        return Err(api_err(
            StatusCode::BAD_REQUEST,
            "only http(s) URLs can be opened",
        ));
    }
    Ok(raw.to_string())
}

async fn open_external(
    app: tauri::AppHandle,
    body: Value,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let raw = body
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| api_err(StatusCode::BAD_REQUEST, "url is required"))?;
    let url = parse_http_url(raw)?;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| api_err(StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(json!({ "ok": true })))
}

async fn check_update(app: tauri::AppHandle) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let updater = app
        .updater()
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Json(json!({
            "available": true,
            "current_version": update.current_version,
            "version": update.version,
            "notes": update.body,
        }))),
        Ok(None) => Ok(Json(json!({
            "available": false,
            "current_version": app.package_info().version.to_string(),
        }))),
        Err(e) => Err(api_err(StatusCode::BAD_GATEWAY, e.to_string())),
    }
}

async fn install_update(app: tauri::AppHandle) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let updater = app
        .updater()
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let update = updater
        .check()
        .await
        .map_err(|e| api_err(StatusCode::BAD_GATEWAY, e.to_string()))?
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "no update available"))?;
    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| api_err(StatusCode::BAD_GATEWAY, e.to_string()))?;
    // Windows installers typically exit the app during install. Elsewhere we relaunch.
    app.restart();
}
