//! Deterministic simulated traffic for demos, tests and offline development.
//! Each aircraft loops: approach from 60 nm → land → taxi → depart outbound.

use super::*;
use crate::geo::destination;

pub struct MockProvider {
    center: (f64, f64),
    scale: f64,
    start: Instant,
}

struct Spec {
    hex: &'static str,
    reg: &'static str,
    callsign: &'static str,
    type_code: &'static str,
    bearing: f64,
    gs: f64,
    phase: f64, // seconds into the loop at t=0
}

const FLEET: &[Spec] = &[
    Spec { hex: "a061d9", reg: "N12345", callsign: "MOCK1", type_code: "C172", bearing: 90.0, gs: 110.0, phase: 1100.0 },
    Spec { hex: "a00001", reg: "N1", callsign: "MOCK2", type_code: "B738", bearing: 200.0, gs: 220.0, phase: 0.0 },
    Spec { hex: "a96ffe", reg: "N707QS", callsign: "MOCK3", type_code: "CL35", bearing: 315.0, gs: 280.0, phase: 600.0 },
    Spec { hex: "ac82ec", reg: "N9999", callsign: "MOCK4", type_code: "PC12", bearing: 45.0, gs: 160.0, phase: 1500.0 },
    Spec { hex: "a4f123", reg: "N5555A", callsign: "MOCK5", type_code: "SR22", bearing: 150.0, gs: 150.0, phase: 300.0 },
];

const START_NM: f64 = 60.0;
const GROUND_SECS: f64 = 60.0;

impl MockProvider {
    pub fn new(center: (f64, f64), scale: f64) -> Self {
        MockProvider { center, scale: scale.max(0.1), start: Instant::now() }
    }

    fn state_at(&self, s: &Spec, sim_secs: f64) -> Aircraft {
        let leg = START_NM / s.gs * 3600.0; // seconds for one approach (or departure) leg
        let cycle = leg * 2.0 + GROUND_SECS;
        let t = (sim_secs + s.phase).rem_euclid(cycle);
        let (dist, track, alt, gs, ground) = if t < leg {
            let d = START_NM - s.gs * t / 3600.0;
            (d, (s.bearing + 180.0) % 360.0, (d * 300.0).clamp(0.0, 12000.0) as i32 + 400, s.gs, false)
        } else if t < leg + GROUND_SECS {
            (0.3, (s.bearing + 180.0) % 360.0, 0, 8.0, true)
        } else {
            let d = s.gs * (t - leg - GROUND_SECS) / 3600.0;
            (d.max(0.4), s.bearing, ((d * 300.0).clamp(0.0, 12000.0) as i32) + 300, s.gs, false)
        };
        let (lat, lon) = destination(self.center.0, self.center.1, s.bearing, dist);
        Aircraft {
            hex: s.hex.into(),
            reg: Some(s.reg.into()),
            callsign: Some(s.callsign.into()),
            type_code: Some(s.type_code.into()),
            lat: Some(lat),
            lon: Some(lon),
            alt_ft: Some(alt),
            on_ground: ground,
            gs_kt: Some(gs),
            track: Some(track),
            baro_rate: Some(0),
            squawk: Some("1200".into()),
            seen_s: Some(0.5),
        }
    }

    pub fn snapshot(&self, sim_secs: f64) -> Vec<Aircraft> {
        FLEET.iter().map(|s| self.state_at(s, sim_secs)).collect()
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn id(&self) -> &'static str {
        "mock"
    }
    fn min_interval(&self) -> Duration {
        Duration::ZERO
    }
    async fn fetch(&self, q: &Query) -> Result<Vec<Aircraft>, ProviderError> {
        let sim = self.start.elapsed().as_secs_f64() * self.scale;
        Ok(super::local::filter(self.snapshot(sim), q))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::distance_nm;

    #[test]
    fn flies_inbound_lands_departs() {
        let m = MockProvider::new((42.3643, -71.0052), 1.0);
        let s = &FLEET[1]; // gs 220 => leg ≈ 982 s
        let d = |t: f64| {
            let a = m.state_at(s, t - s.phase);
            let (la, lo) = a.position().unwrap();
            (distance_nm(42.3643, -71.0052, la, lo), a.on_ground)
        };
        assert!((d(0.0).0 - 60.0).abs() < 0.1);
        assert!(d(500.0).0 < d(100.0).0);
        assert!(d(982.0 + 10.0).1); // on ground
        assert!(d(982.0 + 60.0 + 500.0).0 > 20.0); // outbound
    }
    #[tokio::test]
    async fn query_by_reg() {
        let m = MockProvider::new((0.0, 0.0), 1.0);
        assert_eq!(m.fetch(&Query::Reg("N12345".into())).await.unwrap().len(), 1);
        assert_eq!(m.fetch(&Query::Callsign("MOCK3".into())).await.unwrap().len(), 1);
    }
}
