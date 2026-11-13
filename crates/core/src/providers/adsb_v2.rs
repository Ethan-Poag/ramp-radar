//! The "readsb v2 API" family: adsb.lol, airplanes.live, adsb.fi, ADS-B Exchange (RapidAPI).
//! Attribution: adsb.lol data is ODbL; airplanes.live and adsb.fi have their own terms (see README).

use super::*;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    AdsbLol,
    AirplanesLive,
    AdsbFi,
    AdsbExchange,
}

pub struct AdsbV2 {
    flavor: Flavor,
    http: reqwest::Client,
    secrets: Arc<dyn SecretStore>,
}

impl AdsbV2 {
    pub fn new(flavor: Flavor, http: reqwest::Client, secrets: Arc<dyn SecretStore>) -> Self {
        AdsbV2 {
            flavor,
            http,
            secrets,
        }
    }

    pub fn url(flavor: Flavor, q: &Query) -> String {
        let clamp = |r: f64| r.clamp(1.0, 250.0).round() as u32;
        match flavor {
            Flavor::AdsbLol | Flavor::AirplanesLive => {
                let base = if flavor == Flavor::AdsbLol {
                    "https://api.adsb.lol/v2"
                } else {
                    "https://api.airplanes.live/v2"
                };
                match q {
                    Query::Point {
                        lat,
                        lon,
                        radius_nm,
                    } => format!("{base}/point/{lat:.4}/{lon:.4}/{}", clamp(*radius_nm)),
                    Query::Hex(h) => format!("{base}/hex/{h}"),
                    Query::Reg(r) => format!("{base}/reg/{r}"),
                    Query::Callsign(c) => format!("{base}/callsign/{c}"),
                }
            }
            Flavor::AdsbFi => {
                let base = "https://opendata.adsb.fi/api/v2";
                match q {
                    Query::Point {
                        lat,
                        lon,
                        radius_nm,
                    } => format!(
                        "{base}/lat/{lat:.4}/lon/{lon:.4}/dist/{}",
                        clamp(*radius_nm)
                    ),
                    Query::Hex(h) => format!("{base}/hex/{h}"),
                    Query::Reg(r) => format!("{base}/registration/{r}"),
                    Query::Callsign(c) => format!("{base}/callsign/{c}"),
                }
            }
            Flavor::AdsbExchange => {
                let base = "https://adsbexchange-com1.p.rapidapi.com/v2";
                match q {
                    Query::Point {
                        lat,
                        lon,
                        radius_nm,
                    } => format!(
                        "{base}/lat/{lat:.4}/lon/{lon:.4}/dist/{}/",
                        clamp(*radius_nm)
                    ),
                    Query::Hex(h) => format!("{base}/icao/{h}/"),
                    Query::Reg(r) => format!("{base}/registration/{r}/"),
                    Query::Callsign(c) => format!("{base}/callsign/{c}/"),
                }
            }
        }
    }
}

/// Parse readsb-style JSON (`ac` or `aircraft` array).
pub fn parse_v2(body: &Value) -> Vec<Aircraft> {
    let arr = body
        .get("ac")
        .or_else(|| body.get("aircraft"))
        .and_then(|v| v.as_array());
    let Some(arr) = arr else { return vec![] };
    arr.iter().filter_map(parse_one).collect()
}

fn num(v: &Value, k: &str) -> Option<f64> {
    // some feeds send numbers as strings (ADSBx)
    match v.get(k)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}
fn text(v: &Value, k: &str) -> Option<String> {
    v.get(k)
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn parse_one(v: &Value) -> Option<Aircraft> {
    let hex = text(v, "hex")?.trim_start_matches('~').to_lowercase();
    let (alt_ft, on_ground) = match v.get("alt_baro") {
        Some(Value::String(s)) if s.eq_ignore_ascii_case("ground") => (Some(0), true),
        Some(Value::String(s)) => (s.parse::<f64>().ok().map(|f| f as i32), false),
        Some(Value::Number(n)) => (n.as_f64().map(|f| f as i32), false),
        _ => (None, false),
    };
    Some(Aircraft {
        hex,
        reg: text(v, "r").or_else(|| text(v, "reg")),
        callsign: text(v, "flight"),
        type_code: text(v, "t"),
        lat: num(v, "lat"),
        lon: num(v, "lon"),
        alt_ft,
        on_ground,
        gs_kt: num(v, "gs"),
        track: num(v, "track"),
        baro_rate: num(v, "baro_rate").map(|f| f as i32),
        squawk: text(v, "squawk"),
        seen_s: num(v, "seen"),
    })
}

#[async_trait]
impl Provider for AdsbV2 {
    fn id(&self) -> &'static str {
        match self.flavor {
            Flavor::AdsbLol => "adsb.lol",
            Flavor::AirplanesLive => "airplanes.live",
            Flavor::AdsbFi => "adsb.fi",
            Flavor::AdsbExchange => "adsbexchange",
        }
    }
    fn min_interval(&self) -> Duration {
        match self.flavor {
            Flavor::AirplanesLive | Flavor::AdsbFi => Duration::from_millis(1100), // documented ~1 req/s
            Flavor::AdsbLol => Duration::from_millis(600),
            Flavor::AdsbExchange => Duration::from_secs(2),
        }
    }
    async fn fetch(&self, q: &Query) -> Result<Vec<Aircraft>, ProviderError> {
        let mut req = self.http.get(Self::url(self.flavor, q));
        if self.flavor == Flavor::AdsbExchange {
            let key = self.secrets.get("adsbx_rapidapi_key").ok_or_else(|| {
                ProviderError::Failed(
                    "ADS-B Exchange needs your RapidAPI key (Settings → Secrets)".into(),
                )
            })?;
            req = req
                .header("x-rapidapi-key", key)
                .header("x-rapidapi-host", "adsbexchange-com1.p.rapidapi.com");
        }
        let resp = req.send().await.map_err(http_err)?;
        let status = resp.status();
        if status.as_u16() == 429 {
            let ra = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse().ok());
            return Err(ProviderError::RateLimited(self.id(), ra));
        }
        if !status.is_success() {
            return Err(ProviderError::Failed(format!(
                "{} returned HTTP {}",
                self.id(),
                status.as_u16()
            )));
        }
        let body: Value = resp.json().await.map_err(http_err)?;
        Ok(parse_v2(&body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_real_shape() {
        let b = json!({"ac":[
            {"hex":"a96ffe","flight":"EJA707  ","r":"N707QS","t":"CL35","alt_baro":16350,"gs":389.6,"track":266.91,"lat":42.1,"lon":-71.2,"seen":0.4,"baro_rate":-64},
            {"hex":"~123456","alt_baro":"ground","gs":"3.5","lat":1.0,"lon":2.0},
            {"flight":"nohex"}
        ]});
        let v = parse_v2(&b);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].callsign.as_deref(), Some("EJA707"));
        assert_eq!(v[0].reg.as_deref(), Some("N707QS"));
        assert_eq!(v[0].alt_ft, Some(16350));
        assert!(!v[0].on_ground);
        assert_eq!(v[1].hex, "123456");
        assert!(v[1].on_ground);
        assert_eq!(v[1].gs_kt, Some(3.5));
    }
    #[test]
    fn readsb_aircraft_key() {
        assert_eq!(
            parse_v2(&json!({"aircraft":[{"hex":"abcdef","lat":1,"lon":2}]})).len(),
            1
        );
        assert!(parse_v2(&json!({"error":"x"})).is_empty());
    }
    #[test]
    fn urls() {
        let q = Query::Point {
            lat: 42.36431,
            lon: -71.00521,
            radius_nm: 40.0,
        };
        assert_eq!(
            AdsbV2::url(Flavor::AdsbLol, &q),
            "https://api.adsb.lol/v2/point/42.3643/-71.0052/40"
        );
        assert_eq!(
            AdsbV2::url(Flavor::AdsbFi, &Query::Reg("N1".into())),
            "https://opendata.adsb.fi/api/v2/registration/N1"
        );
        assert!(AdsbV2::url(Flavor::AdsbFi, &q).ends_with("/lat/42.3643/lon/-71.0052/dist/40"));
    }
}
