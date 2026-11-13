//! OurAirports data: download once, cache on disk, search by ICAO/IATA/name/city.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const OURAIRPORTS_URL: &str =
    "https://raw.githubusercontent.com/davidmegginson/ourairports-data/main/airports.csv";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Airport {
    pub ident: String,
    pub iata: Option<String>,
    pub name: String,
    pub kind: String,
    pub city: Option<String>,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    pub elevation_ft: Option<i32>,
}

#[derive(Debug, Default)]
pub struct AirportDb {
    airports: Vec<Airport>,
}

#[derive(Deserialize)]
struct Row {
    ident: String,
    #[serde(rename = "type")]
    kind: String,
    name: String,
    latitude_deg: f64,
    longitude_deg: f64,
    elevation_ft: String,
    iso_country: String,
    municipality: String,
    iata_code: String,
}

/// Small built-in set so the app works offline on first run.
const BUILTIN: &[(&str, &str, &str, &str, f64, f64, i32)] = &[
    ("KBOS", "BOS", "General Edward Lawrence Logan International Airport", "Boston", 42.3643, -71.0052, 20),
    ("PHNL", "HNL", "Daniel K Inouye International Airport", "Honolulu", 21.3187, -157.9224, 13),
    ("PHJR", "JRF", "Kalaeloa Airport", "Kapolei", 21.3075, -158.0703, 30),
    ("PHDH", "HDH", "Dillingham Airfield", "Mokuleia", 21.5795, -158.1970, 14),
    ("KJFK", "JFK", "John F Kennedy International Airport", "New York", 40.6398, -73.7789, 13),
    ("KLAX", "LAX", "Los Angeles International Airport", "Los Angeles", 33.9425, -118.4081, 125),
    ("KSFO", "SFO", "San Francisco International Airport", "San Francisco", 37.6213, -122.3790, 13),
    ("KORD", "ORD", "Chicago O'Hare International Airport", "Chicago", 41.9786, -87.9048, 672),
    ("KATL", "ATL", "Hartsfield Jackson Atlanta International Airport", "Atlanta", 33.6367, -84.4281, 1026),
    ("EGLL", "LHR", "London Heathrow Airport", "London", 51.4706, -0.4619, 83),
];

impl AirportDb {
    pub fn builtin() -> Self {
        AirportDb {
            airports: BUILTIN
                .iter()
                .map(|(i, a, n, c, la, lo, e)| Airport {
                    ident: i.to_string(),
                    iata: Some(a.to_string()),
                    name: n.to_string(),
                    kind: "large_airport".into(),
                    city: Some(c.to_string()),
                    country: if i.starts_with('K') || i.starts_with('P') { "US" } else { "GB" }.into(),
                    lat: *la,
                    lon: *lo,
                    elevation_ft: Some(*e),
                })
                .collect(),
        }
    }

    pub fn from_csv(data: &[u8]) -> Result<Self, csv::Error> {
        let mut rdr = csv::Reader::from_reader(data);
        let mut airports = Vec::new();
        for r in rdr.deserialize::<Row>() {
            let Ok(r) = r else { continue };
            if r.kind == "closed" {
                continue;
            }
            let nz = |s: String| if s.is_empty() { None } else { Some(s) };
            airports.push(Airport {
                ident: r.ident,
                iata: nz(r.iata_code),
                name: r.name,
                kind: r.kind,
                city: nz(r.municipality),
                country: r.iso_country,
                lat: r.latitude_deg,
                lon: r.longitude_deg,
                elevation_ft: r.elevation_ft.parse::<f64>().ok().map(|f| f as i32),
            });
        }
        Ok(AirportDb { airports })
    }

    pub fn len(&self) -> usize {
        self.airports.len()
    }
    pub fn is_empty(&self) -> bool {
        self.airports.is_empty()
    }

    pub fn get(&self, code: &str) -> Option<&Airport> {
        let c = code.trim();
        self.airports
            .iter()
            .find(|a| a.ident.eq_ignore_ascii_case(c))
            .or_else(|| self.airports.iter().find(|a| a.iata.as_deref().is_some_and(|i| i.eq_ignore_ascii_case(c))))
    }

    /// Ranked search: exact ident/IATA > prefix > name/city substring. Bigger airports win ties.
    pub fn search(&self, q: &str, limit: usize) -> Vec<&Airport> {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return vec![];
        }
        let size = |k: &str| match k {
            "large_airport" => 0,
            "medium_airport" => 1,
            "small_airport" => 2,
            _ => 3,
        };
        let mut scored: Vec<(u32, &Airport)> = self
            .airports
            .iter()
            .filter_map(|a| {
                let ident = a.ident.to_lowercase();
                let iata = a.iata.as_deref().unwrap_or("").to_lowercase();
                let rank = if ident == q || iata == q {
                    0
                } else if ident.starts_with(&q) || (!iata.is_empty() && iata.starts_with(&q)) {
                    1
                } else if a.name.to_lowercase().contains(&q)
                    || a.city.as_deref().is_some_and(|c| c.to_lowercase().contains(&q))
                {
                    2
                } else {
                    return None;
                };
                Some((rank * 10 + size(&a.kind), a))
            })
            .collect();
        scored.sort_by_key(|(s, a)| (*s, a.ident.len(), a.ident.clone()));
        scored.into_iter().take(limit).map(|(_, a)| a).collect()
    }
}

/// Load from cache file; download if missing; fall back to the builtin list.
pub async fn load(cache: &Path, http: &reqwest::Client) -> AirportDb {
    if let Ok(bytes) = tokio::fs::read(cache).await {
        if let Ok(db) = AirportDb::from_csv(&bytes) {
            if !db.is_empty() {
                return db;
            }
        }
    }
    match http.get(OURAIRPORTS_URL).send().await.and_then(|r| r.error_for_status()) {
        Ok(resp) => {
            if let Ok(bytes) = resp.bytes().await {
                if let Ok(db) = AirportDb::from_csv(&bytes) {
                    if !db.is_empty() {
                        let _ = tokio::fs::write(cache, &bytes).await;
                        tracing::info!("downloaded {} airports", db.len());
                        return db;
                    }
                }
            }
        }
        Err(e) => tracing::warn!("airport download failed: {e}"),
    }
    AirportDb::builtin()
}

#[cfg(test)]
mod tests {
    use super::*;
    const CSV: &str = "id,ident,type,name,latitude_deg,longitude_deg,elevation_ft,continent,iso_country,iso_region,municipality,scheduled_service,gps_code,iata_code,local_code,home_link,wikipedia_link,keywords\n\
1,KBOS,large_airport,General Edward Lawrence Logan International Airport,42.3643,-71.0052,20,NA,US,US-MA,Boston,yes,KBOS,BOS,BOS,,,\n\
2,KBED,medium_airport,Laurence G Hanscom Field,42.47,-71.289,133,NA,US,US-MA,Bedford,no,KBED,BED,BED,,,\n\
3,XX01,closed,Gone,1,1,,NA,US,US-MA,Nowhere,no,,,,,,\n\
4,PHNL,large_airport,Daniel K Inouye International Airport,21.3187,-157.9224,13,OC,US,US-HI,Honolulu,yes,PHNL,HNL,HNL,,,\n";

    #[test]
    fn parse_and_search() {
        let db = AirportDb::from_csv(CSV.as_bytes()).unwrap();
        assert_eq!(db.len(), 3); // closed skipped
        assert_eq!(db.search("kbos", 5)[0].ident, "KBOS");
        assert_eq!(db.search("hnl", 5)[0].ident, "PHNL"); // IATA
        assert_eq!(db.search("hanscom", 5)[0].ident, "KBED"); // name
        assert_eq!(db.search("honol", 5)[0].ident, "PHNL"); // city
        assert!(db.search("zzzz", 5).is_empty());
        assert_eq!(db.get("BOS").unwrap().ident, "KBOS");
    }
    #[test]
    fn builtin_has_test_airports() {
        let db = AirportDb::builtin();
        assert!(db.get("KBOS").is_some() && db.get("PHNL").is_some());
    }
}
