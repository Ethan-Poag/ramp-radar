use crate::model::ChannelKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfig {
    /// readsb / dump1090 `aircraft.json` over HTTP, e.g. http://pi.local/tar1090/data/aircraft.json
    Local {
        url: String,
        #[serde(default = "yes")]
        enabled: bool,
    },
    AdsbLol {
        #[serde(default = "yes")]
        enabled: bool,
    },
    AirplanesLive {
        #[serde(default = "yes")]
        enabled: bool,
    },
    AdsbFi {
        #[serde(default = "yes")]
        enabled: bool,
    },
    /// OAuth client-credentials: client id here, secret in secret store ("opensky_client_secret")
    OpenSky {
        client_id: String,
        #[serde(default)]
        enabled: bool,
    },
    /// ADS-B Exchange via RapidAPI; key in secret store ("adsbx_rapidapi_key")
    AdsbExchange {
        #[serde(default)]
        enabled: bool,
    },
    Mock {
        #[serde(default)]
        enabled: bool,
    },
}
fn yes() -> bool {
    true
}
impl ProviderConfig {
    pub fn enabled(&self) -> bool {
        match self {
            Self::Local { enabled, .. }
            | Self::AdsbLol { enabled }
            | Self::AirplanesLive { enabled }
            | Self::AdsbFi { enabled }
            | Self::OpenSky { enabled, .. }
            | Self::AdsbExchange { enabled }
            | Self::Mock { enabled } => *enabled,
        }
    }
    pub fn id(&self) -> &'static str {
        match self {
            Self::Local { .. } => "local",
            Self::AdsbLol { .. } => "adsb.lol",
            Self::AirplanesLive { .. } => "airplanes.live",
            Self::AdsbFi { .. } => "adsb.fi",
            Self::OpenSky { .. } => "opensky",
            Self::AdsbExchange { .. } => "adsbexchange",
            Self::Mock { .. } => "mock",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NtfyConfig {
    pub enabled: bool,
    pub server: String,
    pub topic: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EmailConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// "starttls" | "tls" | "none"
    pub security: String,
    pub from: String,
    pub to: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WebhookConfig {
    pub enabled: bool, // URL itself lives in secret store ("webhook_url")
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TwilioConfig {
    pub enabled: bool,
    pub account_sid: String,
    pub from: String,
    pub to: String, // auth token in secret store ("twilio_auth_token")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Notifications {
    pub ntfy: NtfyConfig,
    pub email: EmailConfig,
    pub webhook: WebhookConfig,
    pub twilio: TwilioConfig,
    pub sound: bool,
    pub desktop: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    /// Ordered failover chain.
    pub providers: Vec<ProviderConfig>,
    pub poll_interval_s: u64,
    pub distance_unit: crate::geo::DistUnit,
    pub theme: Theme,
    pub notifications: Notifications,
    /// Hex codes, registrations or callsigns that must never be tracked or displayed.
    pub blocklist: Vec<String>,
    /// Default channels for new trackers
    pub default_channels: Vec<ChannelKind>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            providers: vec![
                ProviderConfig::Local {
                    url: String::new(),
                    enabled: false,
                },
                ProviderConfig::AdsbLol { enabled: true },
                ProviderConfig::AirplanesLive { enabled: true },
                ProviderConfig::AdsbFi { enabled: true },
                ProviderConfig::OpenSky {
                    client_id: String::new(),
                    enabled: false,
                },
                ProviderConfig::AdsbExchange { enabled: false },
                ProviderConfig::Mock { enabled: false },
            ],
            poll_interval_s: 15,
            distance_unit: crate::geo::DistUnit::Nm,
            theme: Theme::System,
            notifications: Notifications {
                ntfy: NtfyConfig {
                    enabled: false,
                    server: "https://ntfy.sh".into(),
                    topic: String::new(),
                },
                email: EmailConfig {
                    port: 587,
                    security: "starttls".into(),
                    ..Default::default()
                },
                sound: true,
                desktop: true,
                ..Default::default()
            },
            blocklist: vec![],
            default_channels: vec![ChannelKind::Desktop, ChannelKind::Sound],
        }
    }
}

impl Settings {
    pub fn is_blocked(&self, hex: &str, reg: Option<&str>, callsign: Option<&str>) -> bool {
        self.blocklist.iter().any(|b| {
            let b = b.trim();
            !b.is_empty()
                && (b.eq_ignore_ascii_case(hex)
                    || reg.is_some_and(|r| b.eq_ignore_ascii_case(r))
                    || callsign.is_some_and(|c| b.eq_ignore_ascii_case(c.trim())))
        })
    }
}
