use crate::db::Db;
use crate::workspace::{Controller, WorkspaceKind, WorkspaceState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Deltager,
    Observer,
}

#[derive(Debug, Clone, Serialize)]
pub struct Participant {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub role: Role,
    pub connected: bool,
    pub minimized: bool,
    /// Hvem sin workspace denne deltageren aktivt kontrollerer (eier-id).
    pub controls: Option<Uuid>,
    /// Hvem som aktivt kontrollerer denne deltagerens workspace.
    pub controlled_by: Option<Uuid>,
    pub workspace_kind: WorkspaceKind,
    pub workspace: WorkspaceState,
    /// Økes ved hver disconnect; brukes til å avbryte utdaterte timeout-tasks.
    #[serde(skip)]
    pub disconnect_gen: u64,
}

/// Klient -> server
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ClientMsg {
    Join {
        name: String,
        role: Role,
        /// Session-token fra tidligere besøk: gjør at man kan komme tilbake
        /// til egen workspace uten å starte ny session.
        session: Option<Uuid>,
    },
    Cursor {
        x: f64,
        y: f64,
    },
    Minimize,
    Restore,
    Control {
        target: Uuid,
    },
    Release,
    Leave,
}

/// Server -> klient
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ServerMsg<'a> {
    Welcome {
        you: &'a Participant,
        session: Uuid,
    },
    Roster {
        participants: Vec<Participant>,
        max_active: usize,
        active_workspaces: usize,
    },
    Cursor {
        id: Uuid,
        color: &'a str,
        name: &'a str,
        x: f64,
        y: f64,
    },
    Error {
        message: String,
    },
}

pub struct Hub {
    inner: Mutex<Inner>,
    pub tx: broadcast::Sender<String>,
    pub controller: Controller,
    pub db: Db,
    pub disconnect_timeout_secs: u64,
}

struct Inner {
    participants: HashMap<Uuid, Participant>,
    /// session-token -> deltager-id, for reconnect/reclaim.
    sessions: HashMap<Uuid, Uuid>,
}

impl Hub {
    pub fn new(controller: Controller, db: Db) -> Arc<Self> {
        let (tx, _) = broadcast::channel(512);
        let disconnect_timeout_secs = std::env::var("WORKSPACE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120);
        Arc::new(Self {
            inner: Mutex::new(Inner {
                participants: HashMap::new(),
                sessions: HashMap::new(),
            }),
            tx,
            controller,
            db,
            disconnect_timeout_secs,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    fn broadcast(&self, msg: String) {
        let _ = self.tx.send(msg);
    }

    /// Full sanntidsstatus som JSON-snapshot.
    pub fn roster_json(&self) -> String {
        let inner = self.inner.lock().unwrap();
        let mut participants: Vec<Participant> = inner.participants.values().cloned().collect();
        participants.sort_by(|a, b| a.name.cmp(&b.name));
        let active_workspaces = participants
            .iter()
            .filter(|p| !matches!(p.workspace, WorkspaceState::None))
            .count();
        serde_json::to_string(&ServerMsg::Roster {
            participants,
            max_active: self.controller.max_active,
            active_workspaces,
        })
        .unwrap()
    }

    /// Send full sanntidsstatus til alle. Kalles ved join/leave/reconnect,
    /// minimize/restore, workspace start/stop og kontroll-endringer.
    pub fn broadcast_roster(&self) {
        let msg = self.roster_json();
        self.broadcast(msg);
    }

    pub fn broadcast_cursor(&self, id: Uuid, x: f64, y: f64) {
        let inner = self.inner.lock().unwrap();
        let Some(p) = inner.participants.get(&id) else {
            return;
        };
        let msg = serde_json::to_string(&ServerMsg::Cursor {
            id,
            color: &p.color,
            name: &p.name,
            x,
            y,
        })
        .unwrap();
        drop(inner);
        self.broadcast(msg);
    }

    /// Join eller reconnect. Returnerer (deltager-id, session-token,
    /// welcome-json, trenger_ny_workspace).
    pub fn join(
        &self,
        name: String,
        role: Role,
        session: Option<Uuid>,
    ) -> (Uuid, Uuid, String, bool) {
        let mut inner = self.inner.lock().unwrap();

        // Reconnect: samme session-token og deltageren finnes fortsatt.
        if let Some(tok) = session {
            if let Some(&pid) = inner.sessions.get(&tok) {
                if let Some(p) = inner.participants.get_mut(&pid) {
                    p.connected = true;
                    p.disconnect_gen += 1; // avbryter ventende timeout
                    p.name = name;
                    p.role = role;
                    let welcome = serde_json::to_string(&ServerMsg::Welcome {
                        you: p,
                        session: tok,
                    })
                    .unwrap();
                    // Provisjonér på nytt både når workspace mangler og når
                    // forrige forsøk feilet — reload av siden blir da retry.
                    let needs_ws = role == Role::Deltager
                        && matches!(
                            p.workspace,
                            WorkspaceState::None | WorkspaceState::Error { .. }
                        );
                    self.db.log_event("reconnect", &p.name);
                    return (pid, tok, welcome, needs_ws);
                }
            }
        }

        // Ny deltager: stabil farge per navn via SQLite, ellers første ledige.
        let in_use: Vec<String> = inner
            .participants
            .values()
            .map(|p| p.color.clone())
            .collect();
        let preferred = self.db.preferred_color(&name);
        let color = crate::colors::pick_color(preferred.as_deref(), &in_use);
        self.db.remember_color(&name, &color);

        let id = Uuid::new_v4();
        let tok = Uuid::new_v4();
        let p = Participant {
            id,
            name: name.clone(),
            color,
            role,
            connected: true,
            minimized: false,
            controls: None,
            controlled_by: None,
            workspace_kind: WorkspaceKind::Chrome,
            workspace: WorkspaceState::None,
            disconnect_gen: 0,
        };
        let welcome =
            serde_json::to_string(&ServerMsg::Welcome { you: &p, session: tok }).unwrap();
        inner.participants.insert(id, p);
        inner.sessions.insert(tok, id);
        self.db.log_event("join", &name);
        (id, tok, welcome, role == Role::Deltager)
    }

    pub fn set_workspace_state(&self, id: Uuid, state: WorkspaceState) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(p) = inner.participants.get_mut(&id) {
            p.workspace = state;
        }
        drop(inner);
        self.broadcast_roster();
    }

    pub fn set_minimized(&self, id: Uuid, minimized: bool) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(p) = inner.participants.get_mut(&id) {
            p.minimized = minimized;
        }
        drop(inner);
        self.broadcast_roster();
    }

    /// Demokratisk kontroll: alle deltagere kan kontrollere hverandres
    /// workspaces, men kun ÉN ekstern aktiv controller per workspace.
    pub fn take_control(&self, who: Uuid, target: Uuid) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        if who == target {
            return Err("du kontrollerer allerede din egen workspace".into());
        }
        match inner.participants.get(&who) {
            Some(p) if p.role == Role::Deltager => {}
            _ => return Err("kun deltagere kan kontrollere andre".into()),
        }
        match inner.participants.get(&target) {
            Some(t) => {
                if !matches!(t.workspace, WorkspaceState::Running { .. }) {
                    return Err("workspacen er ikke aktiv".into());
                }
                if t.controlled_by.is_some() && t.controlled_by != Some(who) {
                    return Err("noen andre kontrollerer denne workspacen allerede".into());
                }
            }
            None => return Err("fant ikke deltageren".into()),
        }
        // Slipp eventuell tidligere kontroll denne deltageren hadde.
        if let Some(prev) = inner.participants.get(&who).and_then(|p| p.controls) {
            if let Some(pt) = inner.participants.get_mut(&prev) {
                if pt.controlled_by == Some(who) {
                    pt.controlled_by = None;
                }
            }
        }
        inner.participants.get_mut(&who).unwrap().controls = Some(target);
        inner.participants.get_mut(&target).unwrap().controlled_by = Some(who);
        drop(inner);
        self.broadcast_roster();
        Ok(())
    }

    pub fn release_control(&self, who: Uuid) {
        let mut inner = self.inner.lock().unwrap();
        let Some(target) = inner
            .participants
            .get_mut(&who)
            .and_then(|p| p.controls.take())
        else {
            return;
        };
        if let Some(t) = inner.participants.get_mut(&target) {
            if t.controlled_by == Some(who) {
                t.controlled_by = None;
            }
        }
        drop(inner);
        self.broadcast_roster();
    }

    pub fn active_workspace_count(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner
            .participants
            .values()
            .filter(|p| !matches!(p.workspace, WorkspaceState::None))
            .count()
    }

    /// Markér frakoblet og returner generasjonsnummeret for timeout-tasken.
    pub fn mark_disconnected(&self, id: Uuid) -> Option<u64> {
        let mut inner = self.inner.lock().unwrap();
        let p = inner.participants.get_mut(&id)?;
        p.connected = false;
        p.disconnect_gen += 1;
        let g = p.disconnect_gen;
        drop(inner);
        self.broadcast_roster();
        Some(g)
    }

    /// True hvis deltageren fortsatt er frakoblet med samme generasjon
    /// (dvs. timeouten er fortsatt gyldig og sessionen skal rives ned).
    pub fn still_disconnected(&self, id: Uuid, generation: u64) -> bool {
        let inner = self.inner.lock().unwrap();
        matches!(
            inner.participants.get(&id),
            Some(p) if !p.connected && p.disconnect_gen == generation
        )
    }

    /// Fjern deltageren helt og rydd opp kontroll-relasjoner.
    /// Returnerer true hvis deltageren hadde en workspace som må destrueres.
    pub fn remove(&self, id: Uuid) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let Some(p) = inner.participants.remove(&id) else {
            return false;
        };
        inner.sessions.retain(|_, v| *v != id);
        for other in inner.participants.values_mut() {
            if other.controls == Some(id) {
                other.controls = None;
            }
            if other.controlled_by == Some(id) {
                other.controlled_by = None;
            }
        }
        drop(inner);
        self.db.log_event("leave", &p.name);
        self.broadcast_roster();
        !matches!(p.workspace, WorkspaceState::None)
    }
}
