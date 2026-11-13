use crate::model::*;
use crate::settings::Settings;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct Store {
    conn: Mutex<Connection>,
}

type R<T> = Result<T, rusqlite::Error>;

impl Store {
    pub fn open(path: &Path) -> R<Self> {
        Self::init(Connection::open(path)?)
    }
    pub fn memory() -> R<Self> {
        Self::init(Connection::open_in_memory()?)
    }
    fn init(conn: Connection) -> R<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS trackers(id TEXT PRIMARY KEY, pos INTEGER NOT NULL, json TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY, json TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS events(
                id INTEGER PRIMARY KEY AUTOINCREMENT, tracker_id TEXT NOT NULL, ts INTEGER NOT NULL,
                kind TEXT NOT NULL, json TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS ev_tracker ON events(tracker_id, ts);",
        )?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    pub fn list_trackers(&self) -> R<Vec<Tracker>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT json FROM trackers ORDER BY pos, rowid")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows
            .filter_map(|j| serde_json::from_str(&j.ok()?).ok())
            .collect())
    }
    pub fn get_tracker(&self, id: &str) -> R<Option<Tracker>> {
        let c = self.conn.lock().unwrap();
        let j: Option<String> = c
            .query_row("SELECT json FROM trackers WHERE id=?", [id], |r| r.get(0))
            .ok();
        Ok(j.and_then(|j| serde_json::from_str(&j).ok()))
    }
    pub fn upsert_tracker(&self, t: &Tracker) -> R<()> {
        let c = self.conn.lock().unwrap();
        let json = serde_json::to_string(t).unwrap();
        let n = c.execute("UPDATE trackers SET json=? WHERE id=?", params![json, t.id])?;
        if n == 0 {
            c.execute(
                "INSERT INTO trackers(id,pos,json) VALUES(?,(SELECT COALESCE(MAX(pos),0)+1 FROM trackers),?)",
                params![t.id, json],
            )?;
        }
        Ok(())
    }
    pub fn delete_tracker(&self, id: &str) -> R<bool> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM events WHERE tracker_id=?", [id])?;
        Ok(c.execute("DELETE FROM trackers WHERE id=?", [id])? > 0)
    }

    pub fn settings(&self) -> Settings {
        let c = self.conn.lock().unwrap();
        c.query_row("SELECT json FROM kv WHERE key='settings'", [], |r| {
            r.get::<_, String>(0)
        })
        .ok()
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default()
    }
    pub fn set_settings(&self, s: &Settings) -> R<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO kv(key,json) VALUES('settings',?) ON CONFLICT(key) DO UPDATE SET json=excluded.json",
            [serde_json::to_string(s).unwrap()],
        )?;
        Ok(())
    }

    pub fn add_event(&self, e: &mut Event) -> R<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO events(tracker_id,ts,kind,json) VALUES(?,?,?,?)",
            params![
                e.tracker_id,
                e.ts.timestamp(),
                format!("{:?}", e.kind).to_lowercase(),
                serde_json::to_string(e).unwrap()
            ],
        )?;
        e.id = c.last_insert_rowid();
        Ok(())
    }
    pub fn events(&self, tracker: Option<&str>, limit: usize) -> R<Vec<Event>> {
        let c = self.conn.lock().unwrap();
        let sql = if tracker.is_some() {
            "SELECT id,json FROM events WHERE tracker_id=?1 ORDER BY ts DESC, id DESC LIMIT ?2"
        } else {
            "SELECT id,json FROM events WHERE ?1 IS NULL ORDER BY ts DESC, id DESC LIMIT ?2"
        };
        let mut stmt = c.prepare(sql)?;
        let rows = stmt.query_map(params![tracker, limit as i64], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        Ok(rows
            .filter_map(|r| {
                let (id, j) = r.ok()?;
                let mut e: Event = serde_json::from_str(&j).ok()?;
                e.id = id;
                Some(e)
            })
            .collect())
    }
}

pub fn events_to_csv(events: &[Event]) -> String {
    fn esc(s: &str) -> String {
        // CSV-injection guard + quoting
        let s = if s.starts_with(['=', '+', '-', '@']) {
            format!("'{s}")
        } else {
            s.to_string()
        };
        format!("\"{}\"", s.replace('"', "\"\""))
    }
    let mut out =
        String::from("time_utc,tracker,aircraft,event,airport,distance_nm,altitude_ft,lat,lon\n");
    for e in events {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            e.ts.to_rfc3339(),
            esc(&e.label),
            esc(&e.aircraft),
            format!("{:?}", e.kind).to_lowercase(),
            esc(&e.airport),
            e.distance_nm.map(|d| format!("{d:.2}")).unwrap_or_default(),
            e.alt_ft.map(|a| a.to_string()).unwrap_or_default(),
            e.lat.map(|d| format!("{d:.5}")).unwrap_or_default(),
            e.lon.map(|d| format!("{d:.5}")).unwrap_or_default(),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::DistUnit;

    pub fn sample(id: &str) -> Tracker {
        Tracker {
            id: id.into(),
            label: "=cmd()".into(),
            target: Target::Reg("N1".into()),
            airport: "KBOS".into(),
            radius: Radius {
                value: 5.0,
                unit: DistUnit::Mi,
            },
            alert_enter: true,
            alert_exit: false,
            alert_landing: true,
            alert_takeoff: false,
            ceiling_ft: Some(3000),
            quiet_hours: None,
            channels: vec![ChannelKind::Ntfy],
            color: "#fff".into(),
            enabled: true,
        }
    }

    #[test]
    fn tracker_crud_and_order() {
        let s = Store::memory().unwrap();
        s.upsert_tracker(&sample("a")).unwrap();
        s.upsert_tracker(&sample("b")).unwrap();
        let mut a = sample("a");
        a.label = "changed".into();
        s.upsert_tracker(&a).unwrap();
        let l = s.list_trackers().unwrap();
        assert_eq!(
            l.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(l[0].label, "changed");
        assert!(s.delete_tracker("a").unwrap());
        assert!(!s.delete_tracker("a").unwrap());
        assert_eq!(s.list_trackers().unwrap().len(), 1);
    }
    #[test]
    fn settings_default_and_roundtrip() {
        let s = Store::memory().unwrap();
        let mut st = s.settings();
        assert_eq!(st.poll_interval_s, 15);
        st.poll_interval_s = 30;
        s.set_settings(&st).unwrap();
        assert_eq!(s.settings().poll_interval_s, 30);
    }
    #[test]
    fn events_and_csv_escape() {
        let s = Store::memory().unwrap();
        let mut e = Event {
            id: 0,
            tracker_id: "a".into(),
            label: "=cmd()".into(),
            ts: chrono::Utc::now(),
            kind: EventKind::Enter,
            airport: "KBOS".into(),
            aircraft: "N1".into(),
            distance_nm: Some(4.2),
            alt_ft: Some(1800),
            lat: Some(1.0),
            lon: Some(2.0),
            alert: true,
        };
        s.add_event(&mut e).unwrap();
        assert!(e.id > 0);
        assert_eq!(s.events(Some("a"), 10).unwrap().len(), 1);
        assert_eq!(s.events(None, 10).unwrap().len(), 1);
        assert_eq!(s.events(Some("zz"), 10).unwrap().len(), 0);
        let csv = events_to_csv(&s.events(None, 10).unwrap());
        assert!(csv.contains("\"'=cmd()\""), "{csv}");
        assert!(csv.contains(",enter,"));
    }
}
