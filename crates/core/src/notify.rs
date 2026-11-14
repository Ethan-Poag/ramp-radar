//! Notification fan-out. Secrets are fetched at send time and never logged; errors are
//! sanitised (no URLs, no credentials).

use crate::model::*;
use crate::secrets::SecretStore;
use crate::settings::Settings;
use serde::Serialize;
use std::sync::{Arc, Mutex};

pub type DesktopHook = Arc<dyn Fn(&str, &str) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
pub struct ChannelResult {
    pub channel: ChannelKind,
    pub ok: bool,
    pub error: Option<String>,
}

/// What goes out to browser/webview clients via SSE (sound / web notification channels).
#[derive(Debug, Clone, Serialize)]
pub struct UiAlert {
    pub title: String,
    pub body: String,
    pub sound: bool,
    pub desktop: bool,
}

pub struct Notifier {
    http: reqwest::Client,
    secrets: Arc<dyn SecretStore>,
    desktop_hook: Mutex<Option<DesktopHook>>,
}

impl Notifier {
    pub fn new(http: reqwest::Client, secrets: Arc<dyn SecretStore>) -> Self {
        Notifier {
            http,
            secrets,
            desktop_hook: Mutex::new(None),
        }
    }
    /// Desktop shell registers a native-notification callback here.
    pub fn set_desktop_hook(&self, hook: DesktopHook) {
        *self.desktop_hook.lock().unwrap() = Some(hook);
    }
    pub fn has_desktop_hook(&self) -> bool {
        self.desktop_hook.lock().unwrap().is_some()
    }

    /// Send to the given channels. `ui` is called once if Sound/Desktop is requested.
    pub async fn send(
        &self,
        channels: &[ChannelKind],
        title: &str,
        body: &str,
        settings: &Settings,
        ui: &(dyn Fn(UiAlert) + Send + Sync),
    ) -> Vec<ChannelResult> {
        let n = &settings.notifications;
        let mut out = Vec::new();
        let want_sound = channels.contains(&ChannelKind::Sound);
        let want_desktop = channels.contains(&ChannelKind::Desktop);
        if want_sound || want_desktop {
            ui(UiAlert {
                title: title.into(),
                body: body.into(),
                sound: want_sound && n.sound,
                desktop: want_desktop && n.desktop,
            });
        }
        for &ch in channels {
            let r: Result<(), String> = match ch {
                ChannelKind::Sound => Ok(()),
                ChannelKind::Desktop => {
                    if !n.desktop {
                        Err("desktop notifications are disabled in Settings".into())
                    } else {
                        if let Some(h) = self.desktop_hook.lock().unwrap().clone() {
                            h(title, body);
                        }
                        Ok(())
                    }
                }
                ChannelKind::Ntfy => self.ntfy(settings, title, body).await,
                ChannelKind::Email => self.email(settings, title, body).await,
                ChannelKind::Webhook => self.webhook(settings, title, body).await,
                ChannelKind::Twilio => self.twilio(settings, title, body).await,
            };
            if let Err(e) = &r {
                tracing::warn!("notification via {ch:?} failed: {e}");
            }
            out.push(ChannelResult {
                channel: ch,
                ok: r.is_ok(),
                error: r.err(),
            });
        }
        out
    }

    async fn ntfy(&self, s: &Settings, title: &str, body: &str) -> Result<(), String> {
        let c = &s.notifications.ntfy;
        if !c.enabled {
            return Err("ntfy is not enabled in Settings".into());
        }
        if c.topic.trim().is_empty() {
            return Err("ntfy topic is empty".into());
        }
        let url = format!("{}/{}", c.server.trim_end_matches('/'), c.topic.trim());
        let mut req = self
            .http
            .post(url)
            .header("Title", ascii(title))
            .header("Tags", "airplane")
            .body(body.to_string());
        if let Some(t) = self.secrets.get("ntfy_token") {
            req = req.bearer_auth(t);
        }
        check(req.send().await).await
    }

    async fn webhook(&self, s: &Settings, title: &str, body: &str) -> Result<(), String> {
        if !s.notifications.webhook.enabled {
            return Err("webhook is not enabled in Settings".into());
        }
        let url = self
            .secrets
            .get("webhook_url")
            .ok_or("webhook URL is not set")?;
        let payload = serde_json::json!({
            "title": title, "message": body, "text": format!("{title}: {body}"),
            "source": "ramp-radar", "ts": chrono::Utc::now().to_rfc3339()
        });
        check(self.http.post(url).json(&payload).send().await).await
    }

    async fn twilio(&self, s: &Settings, title: &str, body: &str) -> Result<(), String> {
        let c = &s.notifications.twilio;
        if !c.enabled {
            return Err("Twilio is not enabled in Settings".into());
        }
        let token = self
            .secrets
            .get("twilio_auth_token")
            .ok_or("Twilio auth token is not set")?;
        if c.account_sid.is_empty() || c.from.is_empty() || c.to.is_empty() {
            return Err("Twilio SID / from / to must be filled in".into());
        }
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            c.account_sid
        );
        let text = format!("{title}: {body}");
        let text: String = text.chars().take(300).collect();
        let r = self
            .http
            .post(url)
            .basic_auth(&c.account_sid, Some(token))
            .form(&[
                ("To", c.to.as_str()),
                ("From", c.from.as_str()),
                ("Body", text.as_str()),
            ])
            .send()
            .await;
        check(r).await
    }

    async fn email(&self, s: &Settings, title: &str, body: &str) -> Result<(), String> {
        use lettre::message::Mailbox;
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
        let c = &s.notifications.email;
        if !c.enabled {
            return Err("email is not enabled in Settings".into());
        }
        let from: Mailbox = c.from.parse().map_err(|_| "invalid From address")?;
        let to: Mailbox = c.to.parse().map_err(|_| "invalid To address")?;
        let msg = Message::builder()
            .from(from)
            .to(to)
            .subject(title)
            .body(body.to_string())
            .map_err(|e| e.to_string())?;
        let mut b = match c.security.as_str() {
            "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&c.host),
            "none" => Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(
                &c.host,
            )),
            _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&c.host),
        }
        .map_err(|e| e.to_string())?
        .port(c.port);
        if !c.username.is_empty() {
            let pw = self
                .secrets
                .get("smtp_password")
                .ok_or("SMTP password is not set")?;
            b = b.credentials(Credentials::new(c.username.clone(), pw));
        }
        b.build()
            .send(msg)
            .await
            .map(|_| ())
            .map_err(|e| format!("SMTP error: {e}"))
    }
}

fn ascii(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() && !c.is_control() {
                c
            } else {
                '-'
            }
        })
        .collect()
}

async fn check(r: Result<reqwest::Response, reqwest::Error>) -> Result<(), String> {
    match r {
        Ok(resp) if resp.status().is_success() => Ok(()),
        Ok(resp) => Err(format!("server returned HTTP {}", resp.status().as_u16())),
        Err(e) => Err(e.without_url().to_string()),
    }
}

/// Should this event be delivered right now? (alert flag + quiet hours.)
pub fn should_deliver(t: &Tracker, e: &Event, minute_of_day: u32) -> bool {
    e.alert && !t.quiet_hours.is_some_and(|q| q.contains(minute_of_day))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::DistUnit;

    fn tr(q: Option<QuietHours>) -> Tracker {
        Tracker {
            id: "x".into(),
            label: "L".into(),
            target: Target::Hex("abcdef".into()),
            airport: "KBOS".into(),
            radius: Radius {
                value: 1.0,
                unit: DistUnit::Nm,
            },
            alert_enter: true,
            alert_exit: false,
            alert_landing: true,
            alert_takeoff: false,
            ceiling_ft: None,
            quiet_hours: q,
            channels: vec![],
            color: "".into(),
            enabled: true,
        }
    }
    fn ev(alert: bool) -> Event {
        Event {
            id: 0,
            tracker_id: "x".into(),
            label: "L".into(),
            ts: chrono::Utc::now(),
            kind: EventKind::Enter,
            airport: "KBOS".into(),
            aircraft: "N1".into(),
            distance_nm: None,
            alt_ft: None,
            lat: None,
            lon: None,
            alert,
        }
    }
    #[test]
    fn quiet_hours_suppress_delivery() {
        let q = Some(QuietHours {
            start_min: 22 * 60,
            end_min: 7 * 60,
        });
        assert!(should_deliver(&tr(q), &ev(true), 12 * 60));
        assert!(!should_deliver(&tr(q), &ev(true), 23 * 60));
        assert!(!should_deliver(&tr(None), &ev(false), 12 * 60));
    }
    #[test]
    fn ascii_titles() {
        assert_eq!(ascii("Bob’s ✈"), "Bob-s -");
    }
}
