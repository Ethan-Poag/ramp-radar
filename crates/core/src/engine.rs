//! Pure proximity state machine. No I/O: easy to test.

use crate::geo::{angle_diff, bearing_deg, distance_nm};
use crate::model::*;
use chrono::{DateTime, Duration, Utc};

/// Exit hysteresis: must be 5 % beyond the radius to count as leaving.
pub const EXIT_HYSTERESIS: f64 = 1.05;
/// Landing/takeoff are attributed to the airport when within this many nm of it.
pub const AIRPORT_VICINITY_NM: f64 = 5.0;
/// Data older than this => NotSeen.
pub const STALE_SECS: i64 = 180;
pub const TRAIL_MAX: usize = 120;

#[derive(Debug, Clone, Copy)]
pub struct AirportPos {
    pub lat: f64,
    pub lon: f64,
}

pub fn evaluate(
    t: &Tracker,
    ap: AirportPos,
    prev: &TrackerState,
    obs: Option<&Aircraft>,
    provider: Option<&str>,
    now: DateTime<Utc>,
) -> (TrackerState, Vec<Event>) {
    let mut st = prev.clone();
    st.tracker_id = t.id.clone();
    let mut events = Vec::new();

    if !t.enabled {
        st.status = Status::Disabled;
        return (st, events);
    }

    // Aircraft with a usable position?
    let fix = obs.and_then(|a| a.position().map(|p| (a, p)));
    let Some((ac, (lat, lon))) = fix else {
        // No fix this poll: go stale after STALE_SECS.
        let stale = st
            .last_seen
            .is_none_or(|ls| now - ls > Duration::seconds(STALE_SECS));
        if stale || st.status == Status::Disabled {
            st.status = Status::NotSeen;
            st.eta_min = None;
        }
        return (st, events);
    };

    let radius = t.radius.nm();
    let dist = distance_nm(lat, lon, ap.lat, ap.lon);
    let prev_dist = prev.distance_nm;
    let on_ground = ac.on_ground;
    let alt = ac.alt_ft;
    let above_ceiling = match (t.ceiling_ft, alt) {
        (Some(c), Some(a)) => !on_ground && a > c,
        _ => false,
    };
    let in_geom = if prev.inside {
        dist <= radius * EXIT_HYSTERESIS
    } else {
        dist <= radius
    };
    let within = in_geom && !above_ceiling;

    let mk = |kind: EventKind, alert: bool| Event {
        id: 0,
        tracker_id: t.id.clone(),
        label: t.label.clone(),
        ts: now,
        kind,
        airport: t.airport.clone(),
        aircraft: ac
            .reg
            .clone()
            .or(ac.callsign.clone())
            .unwrap_or_else(|| ac.hex.to_uppercase()),
        distance_nm: Some(dist),
        alt_ft: alt,
        lat: Some(lat),
        lon: Some(lon),
        alert,
    };

    if !prev.inside && within {
        events.push(mk(EventKind::Enter, t.alert_enter));
    } else if prev.inside && !within {
        events.push(mk(EventKind::Exit, t.alert_exit));
    }
    if dist <= AIRPORT_VICINITY_NM {
        match prev.on_ground {
            Some(false) if on_ground => events.push(mk(EventKind::Landing, t.alert_landing)),
            Some(true) if !on_ground => events.push(mk(EventKind::Takeoff, t.alert_takeoff)),
            _ => {}
        }
    }

    // Closing speed along the line to the airport.
    let brg = bearing_deg(lat, lon, ap.lat, ap.lon);
    let closing_kt = match (ac.gs_kt, ac.track) {
        (Some(gs), Some(tr)) => Some(gs * angle_diff(tr, brg).to_radians().cos()),
        _ => None,
    };
    let approaching = match closing_kt {
        Some(c) if ac.gs_kt.unwrap_or(0.0) > 30.0 => c > 0.0,
        _ => prev_dist.is_some_and(|p| dist < p - 0.05),
    };

    st.status = if on_ground {
        Status::OnGround
    } else if within {
        Status::Within
    } else if approaching {
        Status::Inbound
    } else {
        Status::Outbound
    };
    st.eta_min = match closing_kt {
        Some(c) if c > 30.0 && !on_ground => Some(dist / c * 60.0),
        _ => None,
    };
    st.inside = within;
    st.on_ground = Some(on_ground);
    st.distance_nm = Some(dist);
    st.alt_ft = alt;
    st.gs_kt = ac.gs_kt;
    st.track = ac.track;
    st.lat = Some(lat);
    st.lon = Some(lon);
    st.hex = Some(ac.hex.clone());
    st.reg = ac.reg.clone();
    st.callsign = ac.callsign.clone().map(|c| c.trim().to_string());
    st.type_code = ac.type_code.clone();
    let seen_ago = Duration::milliseconds((ac.seen_s.unwrap_or(0.0) * 1000.0) as i64);
    st.last_seen = Some(now - seen_ago);
    st.provider = provider.map(String::from).or(prev.provider.clone());
    let moved = st.trail.last().is_none_or(|p| (p.lat, p.lon) != (lat, lon));
    if moved {
        st.trail.push(TrailPoint {
            lat,
            lon,
            t: now.timestamp(),
        });
        if st.trail.len() > TRAIL_MAX {
            st.trail.remove(0);
        }
    }
    (st, events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{destination, DistUnit};

    const AP: AirportPos = AirportPos {
        lat: 42.3643,
        lon: -71.0052,
    };

    fn tracker() -> Tracker {
        Tracker {
            id: "t1".into(),
            label: "Test".into(),
            target: Target::Reg("N1".into()),
            airport: "KBOS".into(),
            radius: Radius {
                value: 10.0,
                unit: DistUnit::Nm,
            },
            alert_enter: true,
            alert_exit: true,
            alert_landing: true,
            alert_takeoff: false,
            ceiling_ft: None,
            quiet_hours: None,
            channels: vec![],
            color: "#fff".into(),
            enabled: true,
        }
    }

    /// aircraft `d` nm from airport on bearing 90, flying west (270) toward it
    fn ac_at(d: f64, alt: i32, ground: bool) -> Aircraft {
        let (lat, lon) = destination(AP.lat, AP.lon, 90.0, d);
        Aircraft {
            hex: "abc123".into(),
            lat: Some(lat),
            lon: Some(lon),
            alt_ft: Some(alt),
            on_ground: ground,
            gs_kt: Some(180.0),
            track: Some(270.0),
            ..Default::default()
        }
    }
    fn now() -> DateTime<Utc> {
        chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }
    fn step(t: &Tracker, s: &TrackerState, a: Option<&Aircraft>) -> (TrackerState, Vec<Event>) {
        evaluate(t, AP, s, a, Some("mock"), now())
    }

    #[test]
    fn not_seen_initially() {
        let (s, e) = step(&tracker(), &TrackerState::default(), None);
        assert_eq!(s.status, Status::NotSeen);
        assert!(e.is_empty());
    }

    #[test]
    fn inbound_then_enter_event_once() {
        let t = tracker();
        let (s, e) = step(
            &t,
            &TrackerState::default(),
            Some(&ac_at(30.0, 8000, false)),
        );
        assert_eq!(s.status, Status::Inbound);
        assert!(e.is_empty());
        let eta = s.eta_min.unwrap();
        assert!((eta - 10.0).abs() < 0.5, "{eta}");

        let (s, e) = step(&t, &s, Some(&ac_at(9.0, 4000, false)));
        assert_eq!(s.status, Status::Within);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].kind, EventKind::Enter);
        assert!(e[0].alert);

        // still inside => no duplicate
        let (s, e) = step(&t, &s, Some(&ac_at(8.0, 3500, false)));
        assert_eq!(s.status, Status::Within);
        assert!(e.is_empty());
    }

    #[test]
    fn exit_hysteresis_prevents_flapping() {
        let t = tracker();
        let (s, _) = step(&t, &TrackerState::default(), Some(&ac_at(9.0, 4000, false)));
        assert!(s.inside);
        // 10.3 nm: outside radius but inside the 5 % band => still inside
        let (s, e) = step(&t, &s, Some(&ac_at(10.3, 4000, false)));
        assert!(s.inside && e.is_empty());
        let (s, e) = step(&t, &s, Some(&ac_at(11.0, 4000, false)));
        assert!(!s.inside);
        assert_eq!(e[0].kind, EventKind::Exit);
    }

    #[test]
    fn ceiling_blocks_within() {
        let mut t = tracker();
        t.ceiling_ft = Some(5000);
        let (s, e) = step(
            &t,
            &TrackerState::default(),
            Some(&ac_at(5.0, 12000, false)),
        );
        assert_eq!(s.status, Status::Inbound);
        assert!(e.is_empty());
        let (s, e) = step(&t, &s, Some(&ac_at(4.0, 4000, false)));
        assert_eq!(s.status, Status::Within);
        assert_eq!(e[0].kind, EventKind::Enter);
    }

    #[test]
    fn landing_and_takeoff() {
        let t = tracker();
        let (s, _) = step(&t, &TrackerState::default(), Some(&ac_at(2.0, 300, false)));
        let (s, e) = step(&t, &s, Some(&ac_at(0.5, 20, true)));
        assert_eq!(s.status, Status::OnGround);
        assert!(e.iter().any(|x| x.kind == EventKind::Landing && x.alert));
        let (s2, e) = step(&t, &s, Some(&ac_at(0.5, 100, false)));
        assert!(e.iter().any(|x| x.kind == EventKind::Takeoff && !x.alert));
        assert_ne!(s2.status, Status::OnGround);
    }

    #[test]
    fn far_landing_not_attributed() {
        let t = tracker();
        let (s, _) = step(&t, &TrackerState::default(), Some(&ac_at(50.0, 300, false)));
        let (_, e) = step(&t, &s, Some(&ac_at(50.0, 0, true)));
        assert!(e.is_empty());
    }

    #[test]
    fn outbound_when_moving_away() {
        let t = tracker();
        let mut a = ac_at(20.0, 5000, false);
        a.track = Some(90.0);
        let (s, _) = step(&t, &TrackerState::default(), Some(&a));
        assert_eq!(s.status, Status::Outbound);
        assert!(s.eta_min.is_none());
    }

    #[test]
    fn stale_becomes_not_seen_and_keeps_position() {
        let t = tracker();
        let (s, _) = step(
            &t,
            &TrackerState::default(),
            Some(&ac_at(30.0, 8000, false)),
        );
        // poll with nothing, shortly after: keep status
        let (s1, _) = evaluate(&t, AP, &s, None, None, now() + Duration::seconds(30));
        assert_eq!(s1.status, Status::Inbound);
        let (s2, _) = evaluate(&t, AP, &s, None, None, now() + Duration::seconds(600));
        assert_eq!(s2.status, Status::NotSeen);
        assert!(s2.lat.is_some());
    }

    #[test]
    fn disabled_emits_nothing() {
        let mut t = tracker();
        t.enabled = false;
        let (s, e) = step(&t, &TrackerState::default(), Some(&ac_at(1.0, 100, false)));
        assert_eq!(s.status, Status::Disabled);
        assert!(e.is_empty());
    }

    #[test]
    fn miles_radius_is_converted() {
        let mut t = tracker();
        t.radius = Radius {
            value: 10.0,
            unit: DistUnit::Mi,
        }; // 8.69 nm
        let (s, _) = step(&t, &TrackerState::default(), Some(&ac_at(9.5, 3000, false)));
        assert_ne!(s.status, Status::Within);
        let (s, _) = step(&t, &TrackerState::default(), Some(&ac_at(8.0, 3000, false)));
        assert_eq!(s.status, Status::Within);
    }

    #[test]
    fn trail_is_capped() {
        let t = tracker();
        let mut s = TrackerState::default();
        for i in 0..200 {
            s = step(&t, &s, Some(&ac_at(30.0 - i as f64 * 0.01, 5000, false))).0;
        }
        assert_eq!(s.trail.len(), TRAIL_MAX);
    }

    #[test]
    fn quiet_hours_wrap() {
        let q = QuietHours {
            start_min: 22 * 60,
            end_min: 7 * 60,
        };
        assert!(q.contains(23 * 60));
        assert!(q.contains(3 * 60));
        assert!(!q.contains(12 * 60));
        let q = QuietHours {
            start_min: 9 * 60,
            end_min: 17 * 60,
        };
        assert!(q.contains(10 * 60) && !q.contains(18 * 60));
    }

    #[test]
    fn target_parsing_and_matching() {
        assert_eq!(Target::parse("n800bs"), Some(Target::Reg("N800BS".into())));
        assert_eq!(Target::parse("a4f123"), Some(Target::Hex("a4f123".into())));
        assert_eq!(
            Target::parse("DAL42"),
            Some(Target::Callsign("DAL42".into()))
        );
        assert_eq!(Target::parse("C-FABC"), Some(Target::Reg("C-FABC".into())));
        let ac = Aircraft {
            hex: "a96ffe".into(),
            ..Default::default()
        };
        assert!(Target::Reg("N707QS".into()).matches(&ac)); // by derived hex
    }
}
