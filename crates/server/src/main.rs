use aircraft_monitor_core::{
    api,
    app::{App, AppOptions},
    secrets::FileSecrets,
    store::Store,
};
use clap::Parser;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(version, about = "Aircraft Monitor — headless server")]
struct Cli {
    /// Address to listen on. Use 0.0.0.0:8080 to expose on the LAN (set a token!).
    #[arg(long, env = "AM_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,
    /// Where the database, secrets file and airport cache live.
    #[arg(long, env = "AM_DATA_DIR")]
    data_dir: Option<PathBuf>,
    /// Require this access token (Bearer header, ?token= once, or cookie).
    #[arg(long, env = "AM_ACCESS_TOKEN", hide_env_values = true)]
    access_token: Option<String>,
    /// Enable the simulated "mock" data source in the failover list and make it first.
    #[arg(long, env = "AM_MOCK")]
    mock: bool,
    /// Airport ident the mock traffic is centred on.
    #[arg(long, env = "AM_MOCK_CENTER", default_value = "KBOS")]
    mock_center: String,
    /// Mock time acceleration (e.g. 20 = 20x faster).
    #[arg(long, env = "AM_MOCK_SCALE", default_value_t = 1.0)]
    mock_scale: f64,
    /// Don't download the OurAirports CSV (use cache or tiny built-in list).
    #[arg(long, env = "AM_OFFLINE")]
    offline: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let cli = Cli::parse();

    let data_dir = cli.data_dir.unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("aircraft-monitor"));
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let store = Store::open(&data_dir.join("monitor.db")).expect("open database");
    let secrets = Arc::new(FileSecrets::open(data_dir.join("secrets.json")));

    let center = match aircraft_monitor_core::airports::AirportDb::builtin().get(&cli.mock_center) {
        Some(a) => (a.lat, a.lon),
        None => (42.3643, -71.0052),
    };
    let app = App::new(
        AppOptions {
            data_dir: data_dir.clone(),
            mode: "server",
            access_token: cli.access_token.filter(|t| !t.is_empty()),
            mock_center: center,
            mock_scale: cli.mock_scale,
            download_airports: !cli.offline,
        },
        store,
        secrets,
    )
    .await;

    if cli.mock {
        let mut s = app.settings();
        for p in s.providers.iter_mut() {
            if let aircraft_monitor_core::settings::ProviderConfig::Mock { enabled } = p {
                *enabled = true;
            }
        }
        // mock first so demos are deterministic
        s.providers.sort_by_key(|p| p.id() != "mock");
        app.store.set_settings(&s).expect("save settings");
        app.rebuild_chain();
    }

    if !cli.bind.ip().is_loopback() && app.opts.access_token.is_none() {
        tracing::warn!("listening on {} WITHOUT an access token: anyone on your network can use this. Set AM_ACCESS_TOKEN.", cli.bind);
    }
    app.spawn_poller();
    let listener = tokio::net::TcpListener::bind(cli.bind).await.expect("bind");
    tracing::info!("Aircraft Monitor listening on http://{} (data: {})", cli.bind, data_dir.display());
    axum::serve(listener, api::router(app))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("server");
}
