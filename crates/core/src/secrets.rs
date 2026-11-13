//! Secret storage. Values never leave this module except through `get()` for the
//! code that actually needs them (notifier / provider auth). No `Debug` leak.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub const KNOWN: &[&str] = &[
    "ntfy_token",
    "smtp_password",
    "webhook_url",
    "twilio_auth_token",
    "opensky_client_secret",
    "adsbx_rapidapi_key",
];

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Option<String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
}

/// JSON file with 0600 permissions (server mode).
pub struct FileSecrets {
    path: PathBuf,
    cache: Mutex<BTreeMap<String, String>>,
}

impl std::fmt::Debug for FileSecrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileSecrets({:?}, <redacted>)", self.path)
    }
}

impl FileSecrets {
    pub fn open(path: PathBuf) -> Self {
        let cache = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        FileSecrets { path, cache: Mutex::new(cache) }
    }
    fn persist(&self, map: &BTreeMap<String, String>) -> Result<(), String> {
        use std::io::Write;
        let data = serde_json::to_vec_pretty(map).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("tmp");
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp).map_err(|e| e.to_string())?;
        f.write_all(&data).map_err(|e| e.to_string())?;
        drop(f);
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

impl SecretStore for FileSecrets {
    fn get(&self, name: &str) -> Option<String> {
        self.cache.lock().unwrap().get(name).cloned()
    }
    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let mut m = self.cache.lock().unwrap();
        m.insert(name.to_string(), value.to_string());
        self.persist(&m)
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        let mut m = self.cache.lock().unwrap();
        m.remove(name);
        self.persist(&m)
    }
}

/// In-memory store for tests.
#[derive(Default)]
pub struct MemSecrets(Mutex<BTreeMap<String, String>>);
impl SecretStore for MemSecrets {
    fn get(&self, name: &str) -> Option<String> {
        self.0.lock().unwrap().get(name).cloned()
    }
    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        self.0.lock().unwrap().insert(name.into(), value.into());
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        self.0.lock().unwrap().remove(name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(unix)]
    fn file_is_0600_and_roundtrips() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("am-sec-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("secrets.json");
        let s = FileSecrets::open(p.clone());
        s.set("ntfy_token", "tk_supersecret").unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(FileSecrets::open(p).get("ntfy_token").as_deref(), Some("tk_supersecret"));
        assert!(!format!("{s:?}").contains("supersecret"));
        std::fs::remove_dir_all(dir).ok();
    }
}
