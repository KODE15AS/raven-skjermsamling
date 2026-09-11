mod colors;
mod db;
mod hub;
mod workspace;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use hub::{ClientMsg, Hub, ServerMsg};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "skjermsamling=info,tower_http=info".into()),
        )
        .init();

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "skjermsamling.db".into());
    let db = db::Db::open(&db_path)?;
    let controller = workspace::Controller::from_env();
    let hub = Hub::new(controller, db);

    let static_dir =
        std::env::var("STATIC_DIR").unwrap_or_else(|_| "../frontend/dist".into());
    let index = format!("{static_dir}/index.html");

    // SPA: /, /samling og /wall serveres alle av index.html.
    let spa = ServeDir::new(&static_dir).fallback(ServeFile::new(&index));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/healthz", get(|| async { "ok" }))
        .route("/mock-workspace/:id", get(mock_workspace))
        .fallback_service(spa)
        .with_state(hub.clone());

    let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8015".into());
    let addr: SocketAddr = bind.parse()?;
    info!("skjermsamling lytter på http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(hub): State<Arc<Hub>>,
) -> impl IntoResponse {
    let watch = params.get("watch").map(|v| v == "1").unwrap_or(false);
    ws.on_upgrade(move |socket| async move {
        if watch {
            handle_watch_socket(socket, hub).await;
        } else {
            handle_socket(socket, hub).await;
        }
    })
}

/// Read-only tilkobling for /wall (70"-skjermen): får roster + cursors,
/// men joiner aldri og all innkommende input ignoreres.
async fn handle_watch_socket(socket: WebSocket, hub: Arc<Hub>) {
    use futures_util::{SinkExt, StreamExt};
    let (mut tx_ws, mut rx_ws) = socket.split();
    // Abonner FØR snapshotet sendes, ellers kan endringer mellom snapshot og
    // abonnement gå tapt.
    let mut rx_bcast = hub.subscribe();
    if tx_ws.send(Message::Text(hub.roster_json())).await.is_err() {
        return;
    }
    let forward = tokio::spawn(async move {
        while let Ok(msg) = rx_bcast.recv().await {
            if tx_ws.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });
    // Hold tilkoblingen åpen; dropp alt som kommer inn.
    while let Some(Ok(_)) = rx_ws.next().await {}
    forward.abort();
}

async fn handle_socket(socket: WebSocket, hub: Arc<Hub>) {
    use futures_util::{SinkExt, StreamExt};
    let (mut tx_ws, mut rx_ws) = socket.split();

    // Abonner FØR join fullføres, slik at roster-endringer som skjer i det
    // samme øyeblikket (f.eks. workspace-provisjonering) ikke går tapt.
    let mut rx_bcast = hub.subscribe();

    // Første melding må være join.
    let (id, needs_workspace) = loop {
        let Some(Ok(msg)) = rx_ws.next().await else {
            return;
        };
        let Message::Text(text) = msg else { continue };
        match serde_json::from_str::<ClientMsg>(&text) {
            Ok(ClientMsg::Join {
                name,
                role,
                session,
            }) => {
                let name = name.trim().chars().take(32).collect::<String>();
                if name.is_empty() {
                    let _ = tx_ws
                        .send(Message::Text(
                            serde_json::to_string(&ServerMsg::Error {
                                message: "navn kan ikke være tomt".into(),
                            })
                            .unwrap(),
                        ))
                        .await;
                    continue;
                }
                let (id, _tok, welcome, needs_ws) = hub.join(name, role, session);
                if tx_ws.send(Message::Text(welcome)).await.is_err() {
                    return;
                }
                // Send roster-snapshot direkte til denne socketen, slik at
                // f.eks. en reconnectet /samling-fane har full tilstand
                // umiddelbart selv om ingen nye endringer broadcastes.
                if tx_ws.send(Message::Text(hub.roster_json())).await.is_err() {
                    return;
                }
                break (id, needs_ws);
            }
            _ => continue,
        }
    };

    // Start workspace i bakgrunnen for deltagere uten aktiv workspace.
    if needs_workspace {
        spawn_workspace(hub.clone(), id);
    }
    hub.broadcast_roster();

    // Videresend broadcast-meldinger til denne klienten.
    let forward = tokio::spawn(async move {
        while let Ok(msg) = rx_bcast.recv().await {
            if tx_ws.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Håndter innkommende meldinger.
    let mut explicit_leave = false;
    while let Some(Ok(msg)) = rx_ws.next().await {
        let Message::Text(text) = msg else { continue };
        let Ok(parsed) = serde_json::from_str::<ClientMsg>(&text) else {
            continue;
        };
        match parsed {
            ClientMsg::Cursor { tile, x, y } => hub.broadcast_cursor(id, tile, x, y),
            ClientMsg::Minimize => hub.set_minimized(id, true),
            ClientMsg::Restore => hub.set_minimized(id, false),
            ClientMsg::Control { target } => {
                if let Err(e) = hub.take_control(id, target) {
                    warn!("kontroll avvist: {e}");
                    let _ = hub.tx.send(
                        serde_json::to_string(&ServerMsg::Error { message: e }).unwrap(),
                    );
                }
            }
            ClientMsg::Release => hub.release_control(id),
            ClientMsg::Leave => {
                explicit_leave = true;
                break;
            }
            ClientMsg::Join { .. } => {} // allerede joinet
        }
    }
    forward.abort();

    if explicit_leave {
        teardown(hub, id).await;
        return;
    }

    // Disconnect uten eksplisitt leave: behold sessionen til timeout utløper,
    // slik at deltageren kan komme tilbake uten å starte ny session.
    let Some(generation) = hub.mark_disconnected(id) else {
        return;
    };
    let timeout = hub.disconnect_timeout_secs;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(timeout)).await;
        if hub.still_disconnected(id, generation) {
            info!("timeout utløpt for {id}, river ned session");
            teardown(hub, id).await;
        }
    });
}

fn spawn_workspace(hub: Arc<Hub>, id: Uuid) {
    tokio::spawn(async move {
        if hub.active_workspace_count() >= hub.controller.max_active {
            hub.set_workspace_state(
                id,
                workspace::WorkspaceState::Error {
                    message: format!(
                        "maks {} aktive workspaces – prøv igjen senere",
                        hub.controller.max_active
                    ),
                },
            );
            return;
        }
        hub.set_workspace_state(id, workspace::WorkspaceState::Starting);
        let ws_id = id.to_string();
        match hub.controller.provision(&ws_id).await {
            Ok(state) => hub.set_workspace_state(id, state),
            Err(e) => {
                warn!("workspace-oppstart feilet for {id}: {e:#}");
                let _ = hub.controller.destroy(&ws_id).await;
                hub.set_workspace_state(
                    id,
                    workspace::WorkspaceState::Error {
                        message: "kunne ikke starte workspace".into(),
                    },
                );
            }
        }
    });
}

async fn teardown(hub: Arc<Hub>, id: Uuid) {
    hub.set_workspace_state(id, workspace::WorkspaceState::Stopping);
    let had_workspace = hub.remove(id);
    if had_workspace {
        let ws_id = id.to_string();
        if let Err(e) = hub.controller.stop(&ws_id).await {
            warn!("klarte ikke å stoppe workspace {ws_id}: {e:#}");
        }
        if let Err(e) = hub.controller.destroy(&ws_id).await {
            warn!("klarte ikke å destruere workspace {ws_id}: {e:#}");
        }
    }
}

/// Enkel interaktiv placeholder-side for mock-driveren (utvikling uten Docker).
async fn mock_workspace(Path(id): Path<String>) -> Html<String> {
    let short = id.chars().take(8).collect::<String>();
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Mock workspace</title>
<style>
  body {{ margin:0; height:100vh; display:flex; flex-direction:column; align-items:center;
         justify-content:center; gap:1rem; background:#101418; color:#e8edf2;
         font-family: system-ui, sans-serif; }}
  textarea {{ width:70%; height:40%; background:#181f26; color:#e8edf2;
              border:1px solid #2a333d; border-radius:8px; padding:12px; font-size:15px; }}
</style></head><body>
<h2>Mock Chrome-workspace <code>{short}</code></h2>
<p>Dette er utviklingsdriveren (WORKSPACE_DRIVER=mock). På Raven kjører ekte Chromium her.</p>
<textarea placeholder="Skriv her for å teste input..."></textarea>
</body></html>"#
    ))
}
