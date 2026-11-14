//! Runtime state + poll loop shared by server and desktop.

use crate::airports::{self, AirportDb};
use crate::engine::{evaluate, AirportPos};
use crate::model::*;
use crate::notify::{should_deliver, ChannelResult, Notifier, UiAlert};
use crate::providers::{Chain, Query};
use crate::secrets::SecretStore;
use crate::settings::Settings;
use crate::store::Store;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, Notify};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiMsg {
    State {
        states: Vec<TrackerState>,
        errors: HashMap<String, String>,
    },
    Event {
        event: Event,
    },
    Alert {
        alert: UiAlert,
    },
}

#[derive(Clone)]
pub struct AppOptions {
    pub data_dir: PathBuf,
    /// "server" or "desktop" — the UI uses this to decide who shows native notifications.
    pub mode: &'static str,
    pub access_token: Option<String>,
    pub mock_center: (f64, f64),
    pub mock_scale: f64,
    pub download_airports: bool,
}

type CachedTraffic = (Instant, Arc<Vec<Aircraft>>, &'static str);

pub struct App {
    pub opts: AppOptions,
    pub store: Store,
    pub secrets: Arc<dyn SecretStore>,
    pub http: reqwest::Client,
    pub airports: RwLock<Arc<AirportDb>>,
    pub states: RwLock<HashMap<String, TrackerState>>,
    pub errors: RwLock<HashMap<String, String>>,
    pub chain: RwLock<Arc<Chain>>,
    pub notifier: Notifier,
    pub tx: broadcast::Sender<UiMsg>,
    pub wake: Notify,
    traffic_cache: RwLock<HashMap<String, CachedTraffic>>,
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!(
            "ramp-radar/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/Ethan-Poag/ramp-radar)"
        ))
        .timeout(Duration::from_secs(12))
        .build()
        .expect("http client")
}

impl App {
    pub async fn new(opts: AppOptions, store: Store, secrets: Arc<dyn SecretStore>) -> Arc<App> {
        let http = http_client();
        let airport_db = if opts.download_airports {
            airports::load(&opts.data_dir.join("airports.csv"), &http).await
        } else {
            tokio::fs::read(opts.data_dir.join("airports.csv"))
                .await
                .ok()
                .and_then(|b| AirportDb::from_csv(&b).ok())
                .filter(|d| !d.is_empty())
                .unwrap_or_else(AirportDb::builtin)
        };
        let (tx, _) = broadcast::channel(256);
        let app = Arc::new(App {
            notifier: Notifier::new(http.clone(), secrets.clone()),
            chain: RwLock::new(Arc::new(Chain::new(vec![]))),
            airports: RwLock::new(Arc::new(airport_db)),
            states: Default::default(),
            errors: Default::default(),
            traffic_cache: Default::default(),
            wake: Notify::new(),
            tx,
            http,
            secrets,
            store,
            opts,
        });
        app.rebuild_chain();
        app
    }

    pub fn settings(&self) -> Settings {
        self.store.settings()
    }

    pub fn rebuild_chain(&self) {
        let s = self.settings();
        let chain = Chain::from_settings(
            &s.providers,
            &self.http,
            self.secrets.clone(),
            self.opts.mock_center,
            self.opts.mock_scale,
        );
        *self.chain.write().unwrap() = Arc::new(chain);
        self.traffic_cache.write().unwrap().clear();
    }
    pub fn chain(&self) -> Arc<Chain> {
        self.chain.read().unwrap().clone()
    }
    pub fn airport_db(&self) -> Arc<AirportDb> {
        self.airports.read().unwrap().clone()
    }

    fn query_for(t: &Tracker) -> Query {
        match (&t.target, t.target.known_hex()) {
            (_, Some(h)) => Query::Hex(h),
            (Target::Reg(r), None) => Query::Reg(r.clone()),
            (Target::Callsign(c), None) => Query::Callsign(c.clone()),
            (Target::Hex(h), None) => Query::Hex(h.clone()),
        }
    }

    /// One poll cycle over every tracker. Returns events produced.
    pub async fn poll_once(&self) -> Vec<Event> {
        let trackers = match self.store.list_trackers() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("db: {e}");
                return vec![];
            }
        };
        let settings = self.settings();
        let chain = self.chain();
        let db = self.airport_db();

        // dedupe identical queries across trackers
        let mut results: HashMap<String, Result<(Vec<Aircraft>, &'static str), String>> =
            HashMap::new();
        for t in trackers.iter().filter(|t| t.enabled) {
            let q = Self::query_for(t);
            let k = q.key();
            if let std::collections::hash_map::Entry::Vacant(slot) = results.entry(k) {
                slot.insert(chain.fetch(&q).await.map_err(|e| e.to_string()));
            }
        }

        let now = chrono::Utc::now();
        let mut all_events = Vec::new();
        let mut errors = HashMap::new();
        let mut live: HashMap<String, TrackerState> = HashMap::new();
        for t in &trackers {
            let prev = self
                .states
                .read()
                .unwrap()
                .get(&t.id)
                .cloned()
                .unwrap_or_default();
            let Some(ap) = db.get(&t.airport) else {
                errors.insert(t.id.clone(), format!("unknown airport {}", t.airport));
                continue;
            };
            let (obs, provider) = if t.enabled {
                match results.get(&Self::query_for(t).key()) {
                    Some(Ok((list, p))) => {
                        let best = list
                            .iter()
                            .filter(|a| {
                                t.target.matches(a) || matches!(t.target, Target::Callsign(_))
                            })
                            .filter(|a| {
                                !settings.is_blocked(
                                    &a.hex,
                                    a.reg.as_deref(),
                                    a.callsign.as_deref(),
                                )
                            })
                            .filter(|a| a.position().is_some())
                            .min_by(|a, b| {
                                a.seen_s.unwrap_or(0.0).total_cmp(&b.seen_s.unwrap_or(0.0))
                            });
                        (best.cloned(), Some(*p))
                    }
                    Some(Err(e)) => {
                        errors.insert(t.id.clone(), e.clone());
                        (None, None)
                    }
                    None => (None, None),
                }
            } else {
                (None, None)
            };
            let (st, events) = evaluate(
                t,
                AirportPos {
                    lat: ap.lat,
                    lon: ap.lon,
                },
                &prev,
                obs.as_ref(),
                provider,
                now,
            );
            live.insert(t.id.clone(), st);
            for mut e in events {
                if let Err(err) = self.store.add_event(&mut e) {
                    tracing::error!("db: {err}");
                }
                let _ = self.tx.send(UiMsg::Event { event: e.clone() });
                if should_deliver(t, &e, local_minute_of_day()) {
                    let channels = t.channels.clone();
                    let _ = self
                        .notify(&channels, &e.headline(), &e.body(), &settings)
                        .await;
                }
                all_events.push(e);
            }
        }
        {
            let mut st = self.states.write().unwrap();
            st.retain(|k, _| live.contains_key(k));
            for (k, v) in live {
                st.insert(k, v);
            }
        }
        *self.errors.write().unwrap() = errors;
        self.publish_state();
        all_events
    }

    pub async fn notify(
        &self,
        channels: &[ChannelKind],
        title: &str,
        body: &str,
        settings: &Settings,
    ) -> Vec<ChannelResult> {
        let tx = self.tx.clone();
        self.notifier
            .send(channels, title, body, settings, &move |alert| {
                let _ = tx.send(UiMsg::Alert { alert });
            })
            .await
    }

    pub fn state_snapshot(&self) -> (Vec<TrackerState>, HashMap<String, String>) {
        let order: Vec<String> = self
            .store
            .list_trackers()
            .unwrap_or_default()
            .into_iter()
            .map(|t| t.id)
            .collect();
        let st = self.states.read().unwrap();
        let mut v: Vec<TrackerState> = order
            .iter()
            .map(|id| {
                st.get(id).cloned().unwrap_or(TrackerState {
                    tracker_id: id.clone(),
                    ..Default::default()
                })
            })
            .collect();
        v.shrink_to_fit();
        (v, self.errors.read().unwrap().clone())
    }

    pub fn publish_state(&self) {
        let (states, errors) = self.state_snapshot();
        let _ = self.tx.send(UiMsg::State { states, errors });
    }

    /// Area traffic for the map, cached for 8 s so many browser tabs don't multiply API calls.
    pub async fn traffic(
        &self,
        lat: f64,
        lon: f64,
        radius_nm: f64,
    ) -> Result<(Arc<Vec<Aircraft>>, &'static str), String> {
        let q = Query::Point {
            lat,
            lon,
            radius_nm,
        };
        let key = q.key();
        if let Some((t, v, p)) = self.traffic_cache.read().unwrap().get(&key) {
            if t.elapsed() < Duration::from_secs(8) {
                return Ok((v.clone(), p));
            }
        }
        let settings = self.settings();
        let (list, p) = self.chain().fetch(&q).await.map_err(|e| e.to_string())?;
        let list: Vec<Aircraft> = list
            .into_iter()
            .filter(|a| a.position().is_some())
            .filter(|a| !settings.is_blocked(&a.hex, a.reg.as_deref(), a.callsign.as_deref()))
            .collect();
        let v = Arc::new(list);
        self.traffic_cache
            .write()
            .unwrap()
            .insert(key, (Instant::now(), v.clone(), p));
        Ok((v, p))
    }

    /// Background loop. Wakes early when `wake` is notified (tracker/settings changed).
    pub fn spawn_poller(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let app = self.clone();
        tokio::spawn(async move {
            loop {
                app.poll_once().await;
                let secs = app.settings().poll_interval_s.clamp(5, 3600);
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(secs)) => {}
                    _ = app.wake.notified() => {}
                }
            }
        })
    }
}

fn local_minute_of_day() -> u32 {
    use chrono::Timelike;
    let n = chrono::Local::now();
    n.hour() * 60 + n.minute()
}
