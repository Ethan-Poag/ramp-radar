//! Pluggable ADS-B data providers with failover.

mod adsb_v2;
mod local;
mod mock;
mod opensky;

pub use adsb_v2::{AdsbV2, Flavor};
pub use local::LocalReceiver;
pub use mock::MockProvider;
pub use opensky::OpenSky;

use crate::model::Aircraft;
use crate::secrets::SecretStore;
use crate::settings::ProviderConfig;
use async_trait::async_trait;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Hex(String),
    Reg(String),
    Callsign(String),
    Point { lat: f64, lon: f64, radius_nm: f64 },
}

impl Query {
    pub fn key(&self) -> String {
        match self {
            Query::Hex(h) => format!("hex:{}", h.to_lowercase()),
            Query::Reg(r) => format!("reg:{}", r.to_uppercase()),
            Query::Callsign(c) => format!("cs:{}", c.to_uppercase()),
            Query::Point {
                lat,
                lon,
                radius_nm,
            } => format!("pt:{lat:.3},{lon:.3},{radius_nm:.0}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("{0} does not support this kind of query")]
    Unsupported(&'static str),
    #[error("{0}")]
    Failed(String),
    /// HTTP 429; carries the server's Retry-After (seconds) when given.
    #[error("rate limited by {0}")]
    RateLimited(&'static str, Option<u64>),
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    /// Minimum spacing between calls (rate limit courtesy).
    fn min_interval(&self) -> Duration;
    async fn fetch(&self, q: &Query) -> Result<Vec<Aircraft>, ProviderError>;
}

pub(crate) fn http_err(e: reqwest::Error) -> ProviderError {
    // without_url(): URLs may carry credentials (user:pass@host) for local receivers.
    ProviderError::Failed(e.without_url().to_string())
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Health {
    pub id: String,
    pub ok: Option<bool>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub last_ok_secs_ago: Option<u64>,
    pub backoff_secs: Option<u64>,
}

struct Slot {
    p: Arc<dyn Provider>,
    state: Mutex<SlotState>,
    gate: tokio::sync::Mutex<Option<Instant>>,
}
#[derive(Default)]
struct SlotState {
    fails: u32,
    retry_at: Option<Instant>,
    last_err: Option<String>,
    last_ok: Option<Instant>,
}

/// Ordered failover chain.
pub struct Chain {
    slots: Vec<Slot>,
}

impl Chain {
    pub fn new(providers: Vec<Arc<dyn Provider>>) -> Self {
        Chain {
            slots: providers
                .into_iter()
                .map(|p| Slot {
                    p,
                    state: Default::default(),
                    gate: tokio::sync::Mutex::new(None),
                })
                .collect(),
        }
    }

    pub fn from_settings(
        cfg: &[ProviderConfig],
        http: &reqwest::Client,
        secrets: Arc<dyn SecretStore>,
        mock_center: (f64, f64),
        mock_scale: f64,
    ) -> Self {
        let mut v: Vec<Arc<dyn Provider>> = Vec::new();
        for c in cfg.iter().filter(|c| c.enabled()) {
            match c {
                ProviderConfig::Local { url, .. } if !url.trim().is_empty() => v.push(Arc::new(
                    LocalReceiver::new(http.clone(), url.trim().to_string()),
                )),
                ProviderConfig::Local { .. } => {}
                ProviderConfig::AdsbLol { .. } => v.push(Arc::new(AdsbV2::new(
                    Flavor::AdsbLol,
                    http.clone(),
                    secrets.clone(),
                ))),
                ProviderConfig::AirplanesLive { .. } => v.push(Arc::new(AdsbV2::new(
                    Flavor::AirplanesLive,
                    http.clone(),
                    secrets.clone(),
                ))),
                ProviderConfig::AdsbFi { .. } => v.push(Arc::new(AdsbV2::new(
                    Flavor::AdsbFi,
                    http.clone(),
                    secrets.clone(),
                ))),
                ProviderConfig::AdsbExchange { .. } => v.push(Arc::new(AdsbV2::new(
                    Flavor::AdsbExchange,
                    http.clone(),
                    secrets.clone(),
                ))),
                ProviderConfig::OpenSky { client_id, .. } => v.push(Arc::new(OpenSky::new(
                    http.clone(),
                    client_id.clone(),
                    secrets.clone(),
                ))),
                ProviderConfig::Mock { .. } => {
                    v.push(Arc::new(MockProvider::new(mock_center, mock_scale)))
                }
            }
        }
        Chain::new(v)
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn health(&self) -> Vec<Health> {
        self.slots
            .iter()
            .map(|s| {
                let st = s.state.lock().unwrap();
                Health {
                    id: s.p.id().into(),
                    ok: if st.last_ok.is_none() && st.last_err.is_none() {
                        None
                    } else {
                        Some(st.fails == 0)
                    },
                    consecutive_failures: st.fails,
                    last_error: st.last_err.clone(),
                    last_ok_secs_ago: st.last_ok.map(|t| t.elapsed().as_secs()),
                    backoff_secs: st
                        .retry_at
                        .and_then(|t| t.checked_duration_since(Instant::now()))
                        .map(|d| d.as_secs()),
                }
            })
            .collect()
    }

    /// Try providers in order. Returns (aircraft, provider id).
    pub async fn fetch(&self, q: &Query) -> Result<(Vec<Aircraft>, &'static str), ProviderError> {
        if self.slots.is_empty() {
            return Err(ProviderError::Failed("no data source enabled".into()));
        }
        let mut last: Option<ProviderError> = None;
        let mut tried = false;
        for s in &self.slots {
            let backing_off = s
                .state
                .lock()
                .unwrap()
                .retry_at
                .is_some_and(|t| t > Instant::now());
            if backing_off {
                continue;
            }
            tried = true;
            match self.call(s, q).await {
                Ok(v) => return Ok((v, s.p.id())),
                Err(ProviderError::Unsupported(_)) => continue,
                Err(e) => last = Some(e),
            }
        }
        if !tried {
            return Err(ProviderError::Failed(
                "all data sources are backing off after errors".into(),
            ));
        }
        Err(last.unwrap_or(ProviderError::Failed(
            "no provider supports this query".into(),
        )))
    }

    async fn call(&self, s: &Slot, q: &Query) -> Result<Vec<Aircraft>, ProviderError> {
        {
            // simple per-provider rate limiter
            let mut gate = s.gate.lock().await;
            if let Some(prev) = *gate {
                let wait = s.p.min_interval().saturating_sub(prev.elapsed());
                if !wait.is_zero() {
                    tokio::time::sleep(wait).await;
                }
            }
            *gate = Some(Instant::now());
        }
        let r = s.p.fetch(q).await;
        let mut st = s.state.lock().unwrap();
        match &r {
            Ok(_) => {
                st.fails = 0;
                st.retry_at = None;
                st.last_ok = Some(Instant::now());
                st.last_err = None;
            }
            Err(ProviderError::Unsupported(_)) => {}
            Err(e) => {
                st.fails += 1;
                st.last_err = Some(e.to_string());
                let secs = match e {
                    ProviderError::RateLimited(_, Some(s)) => (*s).clamp(5, 900),
                    _ => (15u64 << (st.fails - 1).min(5)).min(300),
                };
                st.retry_at = Some(Instant::now() + Duration::from_secs(secs));
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        id: &'static str,
        fail: bool,
        calls: Mutex<u32>,
    }
    #[async_trait]
    impl Provider for Fake {
        fn id(&self) -> &'static str {
            self.id
        }
        fn min_interval(&self) -> Duration {
            Duration::ZERO
        }
        async fn fetch(&self, _q: &Query) -> Result<Vec<Aircraft>, ProviderError> {
            *self.calls.lock().unwrap() += 1;
            if self.fail {
                Err(ProviderError::Failed("boom".into()))
            } else {
                Ok(vec![Aircraft {
                    hex: "abc".into(),
                    ..Default::default()
                }])
            }
        }
    }

    #[tokio::test]
    async fn fails_over_and_backs_off() {
        let a = Arc::new(Fake {
            id: "a",
            fail: true,
            calls: Mutex::new(0),
        });
        let b = Arc::new(Fake {
            id: "b",
            fail: false,
            calls: Mutex::new(0),
        });
        let chain = Chain::new(vec![a.clone(), b.clone()]);
        let (v, id) = chain.fetch(&Query::Hex("abc".into())).await.unwrap();
        assert_eq!((v.len(), id), (1, "b"));
        // second call skips "a" because it is backing off
        chain.fetch(&Query::Hex("abc".into())).await.unwrap();
        assert_eq!(*a.calls.lock().unwrap(), 1);
        assert_eq!(*b.calls.lock().unwrap(), 2);
        let h = chain.health();
        assert_eq!(h[0].ok, Some(false));
        assert_eq!(h[1].ok, Some(true));
        assert!(h[0].backoff_secs.is_some());
    }

    #[tokio::test]
    async fn all_failing_is_error() {
        let a = Arc::new(Fake {
            id: "a",
            fail: true,
            calls: Mutex::new(0),
        });
        let chain = Chain::new(vec![a]);
        assert!(chain.fetch(&Query::Hex("x".into())).await.is_err());
        assert!(chain.fetch(&Query::Hex("x".into())).await.is_err()); // backing off
    }
}
