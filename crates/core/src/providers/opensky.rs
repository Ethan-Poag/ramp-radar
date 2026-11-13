//! OpenSky Network REST API. OAuth2 client-credentials when configured, anonymous otherwise.
//! Supports hex and area queries only (OpenSky has no registration/callsign filter).

use super::*;
use crate::geo::destination;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

const TOKEN_URL: &str = "https://auth.opensky-network.org/auth/realms/opensky-network/protocol/openid-connect/token";

pub struct OpenSky {
    http: reqwest::Client,
    client_id: String,
    secrets: Arc<dyn SecretStore>,
    token: AMutex<Option<(String, Instant)>>,
}

impl OpenSky {
    pub fn new(http: reqwest::Client, client_id: String, secrets: Arc<dyn SecretStore>) -> Self {
        OpenSky { http, client_id, secrets, token: AMutex::new(None) }
    }

    async fn bearer(&self) -> Result<Option<String>, ProviderError> {
        let Some(secret) = self.secrets.get("opensky_client_secret").filter(|_| !self.client_id.is_empty()) else {
            return Ok(None);
        };
        let mut guard = self.token.lock().await;
        if let Some((t, exp)) = guard.as_ref() {
            if *exp > Instant::now() {
                return Ok(Some(t.clone()));
            }
        }
        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&[("grant_type", "client_credentials"), ("client_id", self.client_id.as_str()), ("client_secret", secret.as_str())])
            .send()
            .await
            .map_err(http_err)?;
        if !resp.status().is_success() {
            return Err(ProviderError::Failed(format!("OpenSky auth failed: HTTP {}", resp.status().as_u16())));
        }
        let v: Value = resp.json().await.map_err(http_err)?;
        let tok = v["access_token"].as_str().ok_or_else(|| ProviderError::Failed("OpenSky auth: no token".into()))?.to_string();
        let ttl = v["expires_in"].as_u64().unwrap_or(300).saturating_sub(30);
        *guard = Some((tok.clone(), Instant::now() + Duration::from_secs(ttl)));
        Ok(Some(tok))
    }
}

pub fn parse_states(v: &Value) -> Vec<Aircraft> {
    let Some(states) = v.get("states").and_then(|s| s.as_array()) else { return vec![] };
    states
        .iter()
        .filter_map(|s| {
            let a = s.as_array()?;
            let f = |i: usize| a.get(i).and_then(|x| x.as_f64());
            let hex = a.first()?.as_str()?.to_lowercase();
            let on_ground = a.get(8).and_then(|x| x.as_bool()).unwrap_or(false);
            Some(Aircraft {
                hex,
                callsign: a.get(1).and_then(|x| x.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                lon: f(5),
                lat: f(6),
                alt_ft: f(7).or(f(13)).map(|m| (m * 3.28084) as i32),
                on_ground,
                gs_kt: f(9).map(|ms| ms * 1.943_84),
                track: f(10),
                baro_rate: f(11).map(|ms| (ms * 196.85) as i32),
                squawk: a.get(14).and_then(|x| x.as_str()).map(String::from),
                seen_s: f(4).map(|lc| (chrono::Utc::now().timestamp() as f64 - lc).max(0.0)),
                ..Default::default()
            })
        })
        .collect()
}

#[async_trait]
impl Provider for OpenSky {
    fn id(&self) -> &'static str {
        "opensky"
    }
    fn min_interval(&self) -> Duration {
        Duration::from_secs(5) // anonymous users get very few credits; be gentle
    }
    async fn fetch(&self, q: &Query) -> Result<Vec<Aircraft>, ProviderError> {
        let url = match q {
            Query::Hex(h) => format!("https://opensky-network.org/api/states/all?icao24={}", h.to_lowercase()),
            Query::Reg(r) => match crate::nnumber::n_to_icao(r) {
                Some(h) => format!("https://opensky-network.org/api/states/all?icao24={}", h.to_lowercase()),
                None => return Err(ProviderError::Unsupported("opensky")),
            },
            Query::Callsign(_) => return Err(ProviderError::Unsupported("opensky")),
            Query::Point { lat, lon, radius_nm } => {
                let (n, _) = destination(*lat, *lon, 0.0, *radius_nm);
                let (s, _) = destination(*lat, *lon, 180.0, *radius_nm);
                let (_, e) = destination(*lat, *lon, 90.0, *radius_nm);
                let (_, w) = destination(*lat, *lon, 270.0, *radius_nm);
                format!("https://opensky-network.org/api/states/all?lamin={s:.3}&lamax={n:.3}&lomin={w:.3}&lomax={e:.3}")
            }
        };
        let mut req = self.http.get(url);
        if let Some(t) = self.bearer().await? {
            req = req.bearer_auth(t);
        }
        let resp = req.send().await.map_err(http_err)?;
        if !resp.status().is_success() {
            return Err(ProviderError::Failed(format!("opensky returned HTTP {}", resp.status().as_u16())));
        }
        let body: Value = resp.json().await.map_err(http_err)?;
        Ok(parse_states(&body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_state_vector() {
        let v = serde_json::json!({"time":1,"states":[["a96ffe","EJA707  ","United States",1,1,-71.2,42.1,5000.0,false,200.0,266.9,-1.0,null,5100.0,"3417",false,0]]});
        let a = parse_states(&v);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].alt_ft, Some(16404));
        assert!((a[0].gs_kt.unwrap() - 388.77).abs() < 0.1);
        assert_eq!(a[0].callsign.as_deref(), Some("EJA707"));
    }
}
