use crate::geo::DistUnit;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One aircraft observation, normalised across all providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Aircraft {
    pub hex: String,
    pub reg: Option<String>,
    pub callsign: Option<String>,
    pub type_code: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// barometric altitude, feet. None when unknown.
    pub alt_ft: Option<i32>,
    pub on_ground: bool,
    pub gs_kt: Option<f64>,
    pub track: Option<f64>,
    pub baro_rate: Option<i32>,
    pub squawk: Option<String>,
    /// seconds since last message as reported by the provider
    pub seen_s: Option<f64>,
}

impl Aircraft {
    pub fn position(&self) -> Option<(f64, f64)> {
        Some((self.lat?, self.lon?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Target {
    /// Registration / tail number, e.g. N800BS
    Reg(String),
    /// Mode S / ICAO 24-bit hex
    Hex(String),
    /// Flight callsign, e.g. DAL42
    Callsign(String),
}

impl Target {
    /// Parse free text typed by a user: 6 hex chars => Hex, N-number => Reg, otherwise ambiguous
    /// registrations/callsigns are treated as Reg when they contain a dash, else Callsign.
    pub fn parse(input: &str) -> Option<Target> {
        let s = input.trim().to_ascii_uppercase().replace(' ', "");
        if s.is_empty() {
            return None;
        }
        if crate::nnumber::looks_like_n_number(&s) {
            return Some(Target::Reg(s));
        }
        if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(Target::Hex(s.to_ascii_lowercase()));
        }
        if s.contains('-') {
            return Some(Target::Reg(s));
        }
        Some(Target::Callsign(s))
    }
    pub fn display(&self) -> String {
        match self {
            Target::Reg(s) | Target::Callsign(s) => s.clone(),
            Target::Hex(h) => h.to_ascii_uppercase(),
        }
    }
    /// ICAO hex when it can be known without a network lookup.
    pub fn known_hex(&self) -> Option<String> {
        match self {
            Target::Hex(h) => Some(h.to_ascii_lowercase()),
            Target::Reg(r) => crate::nnumber::n_to_icao(r).map(|h| h.to_ascii_lowercase()),
            Target::Callsign(_) => None,
        }
    }
    pub fn matches(&self, ac: &Aircraft) -> bool {
        match self {
            Target::Hex(h) => ac.hex.eq_ignore_ascii_case(h),
            Target::Reg(r) => {
                ac.reg.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(r))
                    || self
                        .known_hex()
                        .is_some_and(|h| ac.hex.eq_ignore_ascii_case(&h))
            }
            Target::Callsign(c) => ac
                .callsign
                .as_deref()
                .is_some_and(|x| x.trim().eq_ignore_ascii_case(c)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Radius {
    pub value: f64,
    #[serde(default)]
    pub unit: DistUnit,
}
impl Radius {
    pub fn nm(&self) -> f64 {
        self.unit.to_nm(self.value)
    }
}

/// Local-time window; may wrap midnight (e.g. 22:00–07:00). Minutes since midnight.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct QuietHours {
    pub start_min: u32,
    pub end_min: u32,
}
impl QuietHours {
    pub fn contains(&self, minute_of_day: u32) -> bool {
        if self.start_min == self.end_min {
            false
        } else if self.start_min < self.end_min {
            (self.start_min..self.end_min).contains(&minute_of_day)
        } else {
            minute_of_day >= self.start_min || minute_of_day < self.end_min
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Desktop,
    Sound,
    Ntfy,
    Email,
    Webhook,
    Twilio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tracker {
    #[serde(default)]
    pub id: String,
    pub label: String,
    pub target: Target,
    /// OurAirports ident (e.g. KBOS)
    pub airport: String,
    pub radius: Radius,
    #[serde(default = "yes")]
    pub alert_enter: bool,
    #[serde(default)]
    pub alert_exit: bool,
    #[serde(default = "yes")]
    pub alert_landing: bool,
    #[serde(default)]
    pub alert_takeoff: bool,
    /// Aircraft above this altitude (ft, baro) is not counted as "within".
    #[serde(default)]
    pub ceiling_ft: Option<i32>,
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
    #[serde(default)]
    pub channels: Vec<ChannelKind>,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "yes")]
    pub enabled: bool,
}
fn yes() -> bool {
    true
}
fn default_color() -> String {
    "#3b82f6".into()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    NotSeen,
    Inbound,
    Within,
    OnGround,
    Outbound,
    Disabled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TrailPoint {
    pub lat: f64,
    pub lon: f64,
    pub t: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackerState {
    pub tracker_id: String,
    pub status: Status,
    pub inside: bool,
    pub on_ground: Option<bool>,
    pub distance_nm: Option<f64>,
    pub eta_min: Option<f64>,
    pub alt_ft: Option<i32>,
    pub gs_kt: Option<f64>,
    pub track: Option<f64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub hex: Option<String>,
    pub reg: Option<String>,
    pub callsign: Option<String>,
    pub type_code: Option<String>,
    pub last_seen: Option<DateTime<Utc>>,
    pub provider: Option<String>,
    pub trail: Vec<TrailPoint>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventKind {
    Enter,
    Exit,
    Landing,
    Takeoff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub id: i64,
    pub tracker_id: String,
    pub label: String,
    pub ts: DateTime<Utc>,
    pub kind: EventKind,
    pub airport: String,
    pub aircraft: String,
    pub distance_nm: Option<f64>,
    pub alt_ft: Option<i32>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Should this event trigger notifications (per the tracker's alert flags)?
    #[serde(default)]
    pub alert: bool,
}

impl Event {
    pub fn headline(&self) -> String {
        let verb = match self.kind {
            EventKind::Enter => "entered the alert radius of",
            EventKind::Exit => "left the alert radius of",
            EventKind::Landing => "landed at",
            EventKind::Takeoff => "departed from",
        };
        format!(
            "{} ({}) {} {}",
            self.label, self.aircraft, verb, self.airport
        )
    }
    pub fn body(&self) -> String {
        let mut s = self.headline();
        if let Some(d) = self.distance_nm {
            s.push_str(&format!(" — {d:.1} nm out"));
        }
        if let Some(a) = self.alt_ft {
            s.push_str(&format!(", {a} ft"));
        }
        s
    }
}
