use clap::Parser;
use open_bevy_protocol::IceServer;
use open_bevy_signaling::{AppState, ServerConfig, TurnRestConfig, build_router};
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(about = "Universal WebRTC signaling service for Open Bevy games")]
struct Args {
    #[arg(long, env = "OPEN_BEVY_BIND", default_value = "0.0.0.0:3536")]
    bind: SocketAddr,

    #[arg(
        long,
        env = "OPEN_BEVY_PUBLIC_WS_BASE",
        default_value = "ws://127.0.0.1:3536"
    )]
    public_ws_base: String,

    #[arg(long, env = "OPEN_BEVY_ROOM_TTL_SECS", default_value_t = 900)]
    room_ttl_secs: u64,

    #[arg(
        long,
        env = "OPEN_BEVY_HOST_RECONNECT_GRACE_SECS",
        default_value_t = 30
    )]
    host_reconnect_grace_secs: u64,

    #[arg(long, env = "OPEN_BEVY_ALLOWED_ORIGINS", default_value = "*")]
    allowed_origins: String,

    #[arg(
        long,
        env = "OPEN_BEVY_ICE_SERVERS_JSON",
        default_value = r#"[{"urls":["stun:stun.l.google.com:19302"]}]"#
    )]
    ice_servers_json: String,

    #[arg(long, env = "OPEN_BEVY_TURN_URLS", default_value = "")]
    turn_urls: String,

    #[arg(long, env = "OPEN_BEVY_TURN_SECRET")]
    turn_secret: Option<String>,

    #[arg(
        long,
        env = "OPEN_BEVY_TURN_CREDENTIAL_TTL_SECS",
        default_value_t = 3600
    )]
    turn_credential_ttl_secs: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("open_bevy_signaling=info,tower_http=info")),
        )
        .init();

    let args = Args::parse();
    let ice_servers = serde_json::from_str::<Vec<IceServer>>(&args.ice_servers_json)?;
    if ice_servers.is_empty() {
        return Err("OPEN_BEVY_ICE_SERVERS_JSON must contain at least one ICE server".into());
    }
    let turn_urls = args
        .turn_urls
        .split(',')
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let turn_rest = match (turn_urls.is_empty(), args.turn_secret) {
        (true, None) => None,
        (false, Some(shared_secret)) if !shared_secret.is_empty() => Some(TurnRestConfig {
            urls: turn_urls,
            shared_secret,
            credential_ttl: Duration::from_secs(args.turn_credential_ttl_secs),
        }),
        _ => {
            return Err(
                "OPEN_BEVY_TURN_URLS and OPEN_BEVY_TURN_SECRET must be configured together".into(),
            );
        }
    };
    let state = AppState::new(ServerConfig {
        public_websocket_base_url: args.public_ws_base,
        ice_servers,
        turn_rest,
        room_ttl: Duration::from_secs(args.room_ttl_secs),
        host_reconnect_grace: Duration::from_secs(args.host_reconnect_grace_secs),
        allowed_origins: args
            .allowed_origins
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_string)
            .collect(),
    });
    let cleanup_state = state.clone();
    let cleanup_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let removed = cleanup_state.remove_expired_rooms().await;
            if removed > 0 {
                info!(removed, "removed expired rooms");
            }
        }
    });

    let listener = TcpListener::bind(args.bind).await?;
    info!(address = %args.bind, "open-bevy signaling server listening");
    let result = axum::serve(listener, build_router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await;
    cleanup_task.abort();
    result?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    warn!("shutdown signal received");
}
