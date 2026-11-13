//! HTTP API + SSE + static UI. The same `router()` is served by the headless server and
//! (on loopback) by the desktop shell.

use crate::app::{App, UiMsg};
use crate::model::*;
use crate::settings::Settings;
use axum::extract::{Path, Query as QS, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use futures_util::stream::{self, Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;

type S = State<Arc<App>>;
type ApiResult<T> = Result<Json<T>, (StatusCode, Json<Value>)>;

fn err(code: StatusCode, msg: impl Into<String>) -> (StatusCode, Json<Value>) {
    (code, Json(json!({ "error": msg.into() })))
}
fn bad(msg: impl Into<String>) -> (StatusCode, Json<Value>) {
    err(StatusCode::BAD_REQUEST, msg)
}
fn db_err(e: rusqlite::Error) -> (StatusCode, Json<Value>) {
    tracing::error!("db: {e}");
    err(StatusCode::INTERNAL_SERVER_ERROR, "database error")
}

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/info", get(info))
        .route("/api/trackers", get(list_trackers).post(create_tracker))
        .route(
            "/api/trackers/{id}",
            get(get_tracker).put(update_tracker).delete(delete_tracker),
        )
        .route("/api/trackers/{id}/test-alert", post(test_alert))
        .route("/api/state", get(state))
        .route("/api/stream", get(stream_sse))
        .route("/api/events", get(events))
        .route("/api/events.csv", get(events_csv))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/secrets", get(list_secrets))
        .route("/api/secrets/{name}", put(set_secret).delete(delete_secret))
        .route("/api/airports", get(search_airports))
        .route("/api/airports/{ident}", get(get_airport))
        .route("/api/traffic", get(traffic))
        .route("/api/lookup", get(lookup))
        .route("/api/providers", get(providers))
        .route("/api/poll", post(poll_now))
        .route("/api/notify/test", post(notify_test))
        .fallback(crate::ui::static_handler)
        .layer(middleware::from_fn_with_state(app.clone(), guard))
        .with_state(app)
}

// ---------- auth / host guard ----------

fn ct_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

fn cookie_token(h: &HeaderMap) -> Option<String> {
    let c = h.get(header::COOKIE)?.to_str().ok()?;
    c.split(';')
        .filter_map(|p| p.trim().strip_prefix("am_token="))
        .next()
        .map(String::from)
}

async fn guard(State(app): S, req: Request, next: Next) -> Response {
    // DNS-rebinding protection for the loopback desktop server.
    if app.opts.mode == "desktop" {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        let name = host.rsplit_once(':').map(|x| x.0).unwrap_or(host);
        if !matches!(name, "127.0.0.1" | "localhost" | "[::1]") {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let Some(token) = app.opts.access_token.clone() else {
        return next.run(req).await;
    };
    let h = req.headers();
    let bearer = h
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(String::from);
    let query_tok = req
        .uri()
        .query()
        .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("token=")))
        .map(|t| t.to_string());
    let given = bearer.or(query_tok.clone()).or_else(|| cookie_token(h));
    if given.as_deref().is_some_and(|g| ct_eq(g, &token)) {
        let mut resp = next.run(req).await;
        if query_tok.is_some() {
            resp.headers_mut().insert(
                header::SET_COOKIE,
                format!("am_token={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000")
                    .parse()
                    .unwrap(),
            );
        }
        return resp;
    }
    if req.uri().path().starts_with("/api/") {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"unauthorized"})),
        )
            .into_response();
    }
    (
        StatusCode::UNAUTHORIZED,
        Html("<!doctype html><meta name=viewport content='width=device-width'><title>Aircraft Monitor</title>\
              <body style='font-family:sans-serif;max-width:24rem;margin:4rem auto'><h2>Aircraft Monitor</h2>\
              <form method=get action=/><input name=token type=password placeholder='Access token' autofocus style='padding:.5rem;width:100%'>\
              <button style='margin-top:.5rem;padding:.5rem 1rem'>Open</button></form>"),
    )
        .into_response()
}

// ---------- handlers ----------

async fn info(State(app): S) -> Json<Value> {
    Json(json!({
        "name": "aircraft-monitor",
        "version": env!("CARGO_PKG_VERSION"),
        "mode": app.opts.mode,
        "auth": app.opts.access_token.is_some(),
        "airports": app.airport_db().len(),
        "native_notifications": app.notifier.has_desktop_hook(),
    }))
}

async fn list_trackers(State(app): S) -> ApiResult<Vec<Tracker>> {
    app.store.list_trackers().map(Json).map_err(db_err)
}
async fn get_tracker(State(app): S, Path(id): Path<String>) -> ApiResult<Tracker> {
    match app.store.get_tracker(&id).map_err(db_err)? {
        Some(t) => Ok(Json(t)),
        None => Err(err(StatusCode::NOT_FOUND, "no such tracker")),
    }
}

fn validate(app: &App, t: &mut Tracker) -> Result<(), (StatusCode, Json<Value>)> {
    t.label = t.label.trim().to_string();
    if t.label.is_empty() || t.label.chars().count() > 80 {
        return Err(bad("label must be 1–80 characters"));
    }
    // normalise target value
    t.target = match &t.target {
        Target::Hex(h) => {
            let h = h.trim().to_lowercase();
            if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(bad("Mode S hex must be 6 hex digits"));
            }
            Target::Hex(h)
        }
        Target::Reg(r) | Target::Callsign(r) if r.trim().is_empty() || r.len() > 12 => {
            return Err(bad("aircraft identifier is required"))
        }
        Target::Reg(r) => Target::Reg(r.trim().to_uppercase()),
        Target::Callsign(c) => Target::Callsign(c.trim().to_uppercase()),
    };
    let r = t.radius.nm();
    if !(0.05..=500.0).contains(&r) {
        return Err(bad("radius must be between 0.05 and 500 nm"));
    }
    if t.ceiling_ft.is_some_and(|c| !(0..=100_000).contains(&c)) {
        return Err(bad("ceiling must be 0–100000 ft"));
    }
    if let Some(q) = t.quiet_hours {
        if q.start_min >= 1440 || q.end_min >= 1440 {
            return Err(bad("invalid quiet hours"));
        }
    }
    let c = &t.color;
    if !(c.starts_with('#')
        && (4..=9).contains(&c.len())
        && c[1..].chars().all(|x| x.is_ascii_hexdigit()))
    {
        t.color = "#3b82f6".into();
    }
    let ap = app.airport_db().get(&t.airport).map(|a| a.ident.clone());
    t.airport = ap.ok_or_else(|| bad("unknown airport"))?;
    let s = app.settings();
    let hex = t.target.known_hex();
    let blocked = match &t.target {
        Target::Hex(h) => s.is_blocked(h, None, None),
        Target::Reg(r) => s.is_blocked(&hex.clone().unwrap_or_default(), Some(r), None),
        Target::Callsign(c) => s.is_blocked("", None, Some(c)),
    };
    if blocked {
        return Err(bad("this aircraft is on your block list"));
    }
    t.channels.sort_by_key(|c| format!("{c:?}"));
    t.channels.dedup();
    Ok(())
}

async fn create_tracker(
    State(app): S,
    Json(mut t): Json<Tracker>,
) -> Result<(StatusCode, Json<Tracker>), (StatusCode, Json<Value>)> {
    t.id = uuid::Uuid::new_v4().to_string();
    validate(&app, &mut t)?;
    app.store.upsert_tracker(&t).map_err(db_err)?;
    app.wake.notify_one();
    Ok((StatusCode::CREATED, Json(t)))
}
async fn update_tracker(
    State(app): S,
    Path(id): Path<String>,
    Json(mut t): Json<Tracker>,
) -> ApiResult<Tracker> {
    if app.store.get_tracker(&id).map_err(db_err)?.is_none() {
        return Err(err(StatusCode::NOT_FOUND, "no such tracker"));
    }
    t.id = id;
    validate(&app, &mut t)?;
    app.store.upsert_tracker(&t).map_err(db_err)?;
    app.wake.notify_one();
    Ok(Json(t))
}
async fn delete_tracker(
    State(app): S,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if app.store.delete_tracker(&id).map_err(db_err)? {
        app.states.write().unwrap().remove(&id);
        app.publish_state();
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(err(StatusCode::NOT_FOUND, "no such tracker"))
    }
}

async fn test_alert(State(app): S, Path(id): Path<String>) -> ApiResult<Value> {
    let t = app
        .store
        .get_tracker(&id)
        .map_err(db_err)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such tracker"))?;
    let e = Event {
        id: 0,
        tracker_id: t.id.clone(),
        label: t.label.clone(),
        ts: chrono::Utc::now(),
        kind: EventKind::Enter,
        airport: t.airport.clone(),
        aircraft: t.target.display(),
        distance_nm: Some(t.radius.nm() * 0.9),
        alt_ft: Some(2500),
        lat: None,
        lon: None,
        alert: true,
    };
    let title = format!("TEST: {}", e.headline());
    let res = app
        .notify(&t.channels, &title, &e.body(), &app.settings())
        .await;
    Ok(Json(json!({ "results": res })))
}

async fn state(State(app): S) -> Json<Value> {
    let (states, errors) = app.state_snapshot();
    Json(json!({ "states": states, "errors": errors }))
}

async fn stream_sse(State(app): S) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let (states, errors) = app.state_snapshot();
    let first = UiMsg::State { states, errors };
    let rx = BroadcastStream::new(app.tx.subscribe());
    let s = stream::once(async move { Ok::<_, Infallible>(first) })
        .chain(rx.filter_map(|m| async move { m.ok().map(Ok) }));
    Sse::new(s.map(|m| {
        let m = m.unwrap();
        let name = match &m {
            UiMsg::State { .. } => "state",
            UiMsg::Event { .. } => "event",
            UiMsg::Alert { .. } => "alert",
        };
        Ok(SseEvent::default()
            .event(name)
            .data(serde_json::to_string(&m).unwrap_or_default()))
    }))
    .keep_alive(KeepAlive::default())
}

#[derive(Deserialize)]
struct EvQ {
    tracker: Option<String>,
    limit: Option<usize>,
}
async fn events(State(app): S, QS(q): QS<EvQ>) -> ApiResult<Vec<Event>> {
    app.store
        .events(q.tracker.as_deref(), q.limit.unwrap_or(200).min(5000))
        .map(Json)
        .map_err(db_err)
}
async fn events_csv(State(app): S, QS(q): QS<EvQ>) -> Result<Response, (StatusCode, Json<Value>)> {
    let ev = app
        .store
        .events(q.tracker.as_deref(), q.limit.unwrap_or(5000).min(50_000))
        .map_err(db_err)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"aircraft-monitor-events.csv\"",
            ),
        ],
        crate::store::events_to_csv(&ev),
    )
        .into_response())
}

async fn get_settings(State(app): S) -> Json<Settings> {
    Json(app.settings())
}
async fn put_settings(State(app): S, Json(mut s): Json<Settings>) -> ApiResult<Settings> {
    if !(5..=3600).contains(&s.poll_interval_s) {
        return Err(bad("poll interval must be 5–3600 s"));
    }
    let srv = s
        .notifications
        .ntfy
        .server
        .trim()
        .trim_end_matches('/')
        .to_string();
    if !srv.is_empty() && !(srv.starts_with("https://") || srv.starts_with("http://")) {
        return Err(bad("ntfy server must start with http:// or https://"));
    }
    s.notifications.ntfy.server = srv;
    s.blocklist = s
        .blocklist
        .iter()
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .collect();
    app.store.set_settings(&s).map_err(db_err)?;
    app.rebuild_chain();
    app.wake.notify_one();
    Ok(Json(s))
}

async fn list_secrets(State(app): S) -> Json<Value> {
    let m: serde_json::Map<String, Value> = crate::secrets::KNOWN
        .iter()
        .map(|k| (k.to_string(), json!(app.secrets.has(k))))
        .collect();
    Json(Value::Object(m))
}
#[derive(Deserialize)]
struct SecretBody {
    value: String,
}
async fn set_secret(
    State(app): S,
    Path(name): Path<String>,
    Json(b): Json<SecretBody>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if !crate::secrets::KNOWN.contains(&name.as_str()) {
        return Err(bad("unknown secret name"));
    }
    if b.value.is_empty() || b.value.len() > 2048 {
        return Err(bad("secret value must be 1–2048 characters"));
    }
    app.secrets
        .set(&name, &b.value)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "could not store secret"))?;
    app.rebuild_chain();
    Ok(StatusCode::NO_CONTENT)
}
async fn delete_secret(
    State(app): S,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if !crate::secrets::KNOWN.contains(&name.as_str()) {
        return Err(bad("unknown secret name"));
    }
    app.secrets
        .delete(&name)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "could not delete secret"))?;
    app.rebuild_chain();
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct AQ {
    q: String,
    limit: Option<usize>,
}
async fn search_airports(State(app): S, QS(q): QS<AQ>) -> Json<Value> {
    let db = app.airport_db();
    Json(json!(db.search(&q.q, q.limit.unwrap_or(10).min(50))))
}
async fn get_airport(State(app): S, Path(ident): Path<String>) -> ApiResult<Value> {
    app.airport_db()
        .get(&ident)
        .map(|a| Json(json!(a)))
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "unknown airport"))
}

#[derive(Deserialize)]
struct TQ {
    lat: f64,
    lon: f64,
    radius: Option<f64>,
}
async fn traffic(State(app): S, QS(q): QS<TQ>) -> ApiResult<Value> {
    if !(-90.0..=90.0).contains(&q.lat) || !(-180.0..=180.0).contains(&q.lon) {
        return Err(bad("bad coordinates"));
    }
    let (list, provider) = app
        .traffic(q.lat, q.lon, q.radius.unwrap_or(40.0).clamp(1.0, 250.0))
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, e))?;
    Ok(Json(json!({ "provider": provider, "aircraft": *list })))
}

#[derive(Deserialize)]
struct LQ {
    q: String,
}
/// Parse what a user typed: N-number, hex or callsign, and show the derived hex/N-number.
async fn lookup(QS(q): QS<LQ>) -> Json<Value> {
    let t = Target::parse(&q.q);
    let (hex, reg) = match &t {
        Some(Target::Reg(r)) => (crate::nnumber::n_to_icao(r), None),
        Some(Target::Hex(h)) => (None, crate::nnumber::icao_to_n(h)),
        _ => (None, None),
    };
    Json(json!({ "target": t, "hex": hex.map(|h| h.to_lowercase()), "n_number": reg }))
}

async fn providers(State(app): S) -> Json<Value> {
    Json(json!(app.chain().health()))
}
async fn poll_now(State(app): S) -> StatusCode {
    app.wake.notify_one();
    StatusCode::ACCEPTED
}

#[derive(Deserialize)]
struct NT {
    channel: ChannelKind,
}
async fn notify_test(State(app): S, Json(b): Json<NT>) -> Json<Value> {
    let res = app
        .notify(
            &[b.channel],
            "TEST: Aircraft Monitor",
            "If you can read this, this channel works.",
            &app.settings(),
        )
        .await;
    Json(json!({ "results": res }))
}
