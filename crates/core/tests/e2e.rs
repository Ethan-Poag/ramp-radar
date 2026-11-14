//! End-to-end: mock provider -> poll -> engine -> SQLite -> ntfy delivery (to a local fake ntfy).

use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use ramp_radar_core::{
    api,
    app::{App, AppOptions},
    model::*,
    secrets::{MemSecrets, SecretStore},
    settings::ProviderConfig,
    store::Store,
};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const KBOS: (f64, f64) = (42.3643, -71.0052);

type Hits = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

async fn fake_ntfy() -> (String, Hits) {
    let hits: Hits = Default::default();
    let h = hits.clone();
    let app = axum::Router::new().route(
        "/{topic}",
        axum::routing::post(
            move |axum::extract::Path(topic): axum::extract::Path<String>,
                  headers: axum::http::HeaderMap,
                  body: String| {
                let h = h.clone();
                async move {
                    let auth = headers
                        .get("authorization")
                        .map(|v| v.to_str().unwrap().to_string());
                    h.lock().unwrap().push((topic, body, auth));
                    "ok"
                }
            },
        ),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (url, hits)
}

async fn make_app(
    scale: f64,
    mode: &'static str,
    token: Option<&str>,
) -> (Arc<App>, Arc<MemSecrets>) {
    let secrets = Arc::new(MemSecrets::default());
    let dir = std::env::temp_dir().join(format!("am-e2e-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let app = App::new(
        AppOptions {
            data_dir: dir,
            mode,
            access_token: token.map(String::from),
            mock_center: KBOS,
            mock_scale: scale,
            download_airports: false,
        },
        Store::memory().unwrap(),
        secrets.clone(),
    )
    .await;
    let mut s = app.settings();
    s.providers = vec![ProviderConfig::Mock { enabled: true }];
    app.store.set_settings(&s).unwrap();
    app.rebuild_chain();
    (app, secrets)
}

fn tracker(target: Target) -> Tracker {
    serde_json::from_value(serde_json::json!({
        "label": "Test bird", "target": target, "airport": "KBOS",
        "radius": {"value": 10.0, "unit": "nm"}, "channels": ["ntfy", "sound"], "alert_exit": true
    }))
    .unwrap()
}

async fn call(
    app: &Arc<App>,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (u16, serde_json::Value) {
    let mut r = Request::builder().method(method).uri(uri);
    let b = match body {
        Some(v) => {
            r = r.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = api::router(app.clone())
        .oneshot(r.body(b).unwrap())
        .await
        .unwrap();
    let code = resp.status().as_u16();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        code,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

/// MOCK3 (N707QS) starts 600 s into its 60 nm / 280 kt loop => ~ 21 nm out, inbound.
/// With 100x time acceleration it enters the 10 nm radius within ~2 s of wall time.
#[tokio::test]
async fn proximity_alert_reaches_ntfy() {
    let (app, secrets) = make_app(100.0, "server", None).await;
    let (url, hits) = fake_ntfy().await;
    secrets.set("ntfy_token", "tk_secret_value").unwrap();
    let mut s = app.settings();
    s.notifications.ntfy.enabled = true;
    s.notifications.ntfy.server = url;
    s.notifications.ntfy.topic = "plane-test".into();
    app.store.set_settings(&s).unwrap();

    let mut t = tracker(Target::Reg("N707QS".into()));
    t.id = "t1".into();
    app.store.upsert_tracker(&t).unwrap();

    let mut all = vec![];
    for _ in 0..40 {
        all.extend(app.poll_once().await);
        if all.iter().any(|e| e.kind == EventKind::Enter) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let enter = all
        .iter()
        .find(|e| e.kind == EventKind::Enter)
        .expect("enter event");
    assert!(enter.alert && enter.distance_nm.unwrap() <= 10.0);

    let hits = hits.lock().unwrap();
    assert_eq!(hits.len(), 1, "exactly one ntfy message");
    assert_eq!(hits[0].0, "plane-test");
    assert!(
        hits[0].1.contains("entered the alert radius of KBOS"),
        "{}",
        hits[0].1
    );
    assert_eq!(hits[0].2.as_deref(), Some("Bearer tk_secret_value"));

    // persisted
    let ev = app.store.events(Some("t1"), 10).unwrap();
    assert_eq!(ev[0].kind, EventKind::Enter);
    // live state shows within
    let (states, _) = app.state_snapshot();
    assert_eq!(states[0].status, Status::Within);
}

#[tokio::test]
async fn api_crud_and_validation() {
    let (app, _) = make_app(1.0, "server", None).await;
    let t = serde_json::to_value(tracker(Target::Reg("n12345".into()))).unwrap();
    let (c, created) = call(&app, "POST", "/api/trackers", Some(t.clone())).await;
    assert_eq!(c, 201);
    assert_eq!(created["target"]["value"], "N12345");
    let id = created["id"].as_str().unwrap().to_string();

    let (c, list) = call(&app, "GET", "/api/trackers", None).await;
    assert_eq!((c, list.as_array().unwrap().len()), (200, 1));

    let mut upd = created.clone();
    upd["label"] = "Renamed".into();
    let (c, u) = call(&app, "PUT", &format!("/api/trackers/{id}"), Some(upd)).await;
    assert_eq!((c, u["label"].as_str()), (200, Some("Renamed")));

    // validation
    let mut badt = t.clone();
    badt["airport"] = "ZZZZ".into();
    assert_eq!(call(&app, "POST", "/api/trackers", Some(badt)).await.0, 400);
    let mut badt = t.clone();
    badt["radius"]["value"] = 0.0.into();
    assert_eq!(call(&app, "POST", "/api/trackers", Some(badt)).await.0, 400);

    // block list refuses
    let mut s = app.settings();
    s.blocklist = vec!["n12345".into()];
    let (c, _) = call(
        &app,
        "PUT",
        "/api/settings",
        Some(serde_json::to_value(&s).unwrap()),
    )
    .await;
    assert_eq!(c, 200);
    assert_eq!(call(&app, "POST", "/api/trackers", Some(t)).await.0, 400);

    assert_eq!(
        call(&app, "DELETE", &format!("/api/trackers/{id}"), None)
            .await
            .0,
        204
    );
    assert_eq!(
        call(&app, "GET", &format!("/api/trackers/{id}"), None)
            .await
            .0,
        404
    );
}

#[tokio::test]
async fn secrets_are_write_only() {
    let (app, _) = make_app(1.0, "server", None).await;
    let (c, _) = call(
        &app,
        "PUT",
        "/api/secrets/ntfy_token",
        Some(serde_json::json!({"value": "hunter2-very-secret"})),
    )
    .await;
    assert_eq!(c, 204);
    let (_, listed) = call(&app, "GET", "/api/secrets", None).await;
    assert_eq!(listed["ntfy_token"], true);
    assert_eq!(listed["smtp_password"], false);
    assert!(!listed.to_string().contains("hunter2"));
    let (_, settings) = call(&app, "GET", "/api/settings", None).await;
    assert!(!settings.to_string().contains("hunter2"));
    assert_eq!(
        call(
            &app,
            "PUT",
            "/api/secrets/not_a_secret",
            Some(serde_json::json!({"value": "x"}))
        )
        .await
        .0,
        400
    );
}

#[tokio::test]
async fn lookup_and_airports() {
    let (app, _) = make_app(1.0, "server", None).await;
    let (_, v) = call(&app, "GET", "/api/lookup?q=n12345", None).await;
    assert_eq!(v["hex"], "a061d9");
    let (_, v) = call(&app, "GET", "/api/lookup?q=A061D9", None).await;
    assert_eq!(v["n_number"], "N12345");
    let (_, v) = call(&app, "GET", "/api/airports?q=boston", None).await;
    assert_eq!(v[0]["ident"], "KBOS");
}

#[tokio::test]
async fn access_token_guard() {
    let (app, _) = make_app(1.0, "server", Some("s3cret")).await;
    assert_eq!(call(&app, "GET", "/api/info", None).await.0, 401);
    let resp = api::router(app.clone())
        .oneshot(
            Request::builder()
                .uri("/api/info")
                .header("authorization", "Bearer s3cret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let resp = api::router(app.clone())
        .oneshot(
            Request::builder()
                .uri("/?token=s3cret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("rr_token=s3cret"));
    let resp = api::router(app.clone())
        .oneshot(
            Request::builder()
                .uri("/api/info")
                .header("cookie", "rr_token=wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn desktop_mode_rejects_foreign_host() {
    let (app, _) = make_app(1.0, "desktop", None).await;
    let ok = api::router(app.clone())
        .oneshot(
            Request::builder()
                .uri("/api/info")
                .header("host", "127.0.0.1:5555")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    let evil = api::router(app.clone())
        .oneshot(
            Request::builder()
                .uri("/api/info")
                .header("host", "evil.example:5555")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(evil.status(), 403);
}
