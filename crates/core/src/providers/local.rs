//! Local receiver: readsb / dump1090 / tar1090 `aircraft.json` over HTTP.
//! (SBS/Beast TCP is on the roadmap.)

use super::adsb_v2::parse_v2;
use super::*;
use crate::geo::distance_nm;

pub struct LocalReceiver {
    http: reqwest::Client,
    url: String,
}
impl LocalReceiver {
    pub fn new(http: reqwest::Client, url: String) -> Self {
        LocalReceiver { http, url }
    }
}

pub fn filter(all: Vec<Aircraft>, q: &Query) -> Vec<Aircraft> {
    all.into_iter()
        .filter(|a| match q {
            Query::Hex(h) => a.hex.eq_ignore_ascii_case(h),
            Query::Reg(r) => {
                a.reg.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(r))
                    || crate::nnumber::n_to_icao(r).is_some_and(|h| a.hex.eq_ignore_ascii_case(&h))
            }
            Query::Callsign(c) => a
                .callsign
                .as_deref()
                .is_some_and(|x| x.eq_ignore_ascii_case(c)),
            Query::Point {
                lat,
                lon,
                radius_nm,
            } => a
                .position()
                .is_some_and(|(la, lo)| distance_nm(*lat, *lon, la, lo) <= *radius_nm),
        })
        .collect()
}

#[async_trait]
impl Provider for LocalReceiver {
    fn id(&self) -> &'static str {
        "local"
    }
    fn min_interval(&self) -> Duration {
        Duration::from_millis(500)
    }
    async fn fetch(&self, q: &Query) -> Result<Vec<Aircraft>, ProviderError> {
        let resp = self.http.get(&self.url).send().await.map_err(http_err)?;
        if !resp.status().is_success() {
            return Err(ProviderError::Failed(format!(
                "local receiver returned HTTP {}",
                resp.status().as_u16()
            )));
        }
        let body: serde_json::Value = resp.json().await.map_err(http_err)?;
        Ok(filter(parse_v2(&body), q))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters() {
        let mk = |hex: &str, lat: f64| Aircraft {
            hex: hex.into(),
            lat: Some(lat),
            lon: Some(0.0),
            ..Default::default()
        };
        let all = vec![mk("a96ffe", 0.0), mk("111111", 5.0)];
        assert_eq!(filter(all.clone(), &Query::Reg("N707QS".into())).len(), 1);
        assert_eq!(
            filter(
                all.clone(),
                &Query::Point {
                    lat: 0.0,
                    lon: 0.0,
                    radius_nm: 10.0
                }
            )
            .len(),
            1
        );
        assert_eq!(
            filter(
                all,
                &Query::Point {
                    lat: 0.0,
                    lon: 0.0,
                    radius_nm: 400.0
                }
            )
            .len(),
            2
        );
    }
}
