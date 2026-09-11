use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::time::Duration;
use tokio::process::Command;

/// Intern abstraksjon: en `Workspace` er en fjernstyrbar arbeidsflate.
/// I MVP finnes kun én type (Chromium + Selkies/KasmVNC i container),
/// men modellen er typet slik at flere typer kan legges til senere
/// uten å endre produktmodellen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceKind {
    Chrome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum WorkspaceState {
    None,
    Starting,
    Running { url: String },
    Stopping,
    Error { message: String },
}

/// Session-controlleren er den ENESTE komponenten som får starte/stoppe
/// workspaces, og den støtter kun de faste operasjonene
/// create / start / stop / status / destroy. Frontend kan aldri sende
/// Docker-parametere; alt her er bygget fra server-side konfig.
pub struct Controller {
    driver: Driver,
    pub max_active: usize,
}

pub enum Driver {
    /// Kjører faste `docker`-CLI-kommandoer på Raven.
    Cli(CliDriver),
    /// Utviklings-/testdriver uten Docker: peker på en innebygd mock-side.
    Mock,
}

pub struct CliDriver {
    image: String,
    memory: String,
    cpus: String,
    shm_size: String,
    /// Vertsnavn klientene skal nå workspacen på. Tom streng betyr at
    /// frontend erstatter `{host}` med window.location.hostname.
    public_host: String,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Controller {
    pub fn from_env() -> Self {
        let max_active: usize = env_or("MAX_ACTIVE_WORKSPACES", "4").parse().unwrap_or(4);
        let driver = match env_or("WORKSPACE_DRIVER", "docker").as_str() {
            "mock" => Driver::Mock,
            _ => Driver::Cli(CliDriver {
                image: env_or("WORKSPACE_IMAGE", "lscr.io/linuxserver/chromium:latest"),
                memory: env_or("WORKSPACE_MEMORY", "3g"),
                cpus: env_or("WORKSPACE_CPUS", "2"),
                shm_size: env_or("WORKSPACE_SHM_SIZE", "1g"),
                public_host: env_or("WORKSPACE_PUBLIC_HOST", ""),
            }),
        };
        Self { driver, max_active }
    }

    fn container_name(ws_id: &str) -> String {
        format!("skjermsamling-ws-{ws_id}")
    }

    // ---- De fem faste operasjonene ----

    pub async fn create(&self, ws_id: &str) -> Result<()> {
        match &self.driver {
            Driver::Mock => Ok(()),
            Driver::Cli(d) => d.create(&Self::container_name(ws_id)).await,
        }
    }

    pub async fn start(&self, ws_id: &str) -> Result<()> {
        match &self.driver {
            Driver::Mock => Ok(()),
            Driver::Cli(_) => {
                docker(&["start", &Self::container_name(ws_id)]).await?;
                Ok(())
            }
        }
    }

    /// Returnerer Running med URL når workspacen er klar.
    pub async fn status(&self, ws_id: &str) -> Result<WorkspaceState> {
        match &self.driver {
            Driver::Mock => Ok(WorkspaceState::Running {
                url: format!("/mock-workspace/{ws_id}"),
            }),
            Driver::Cli(d) => d.status(&Self::container_name(ws_id)).await,
        }
    }

    pub async fn stop(&self, ws_id: &str) -> Result<()> {
        match &self.driver {
            Driver::Mock => Ok(()),
            Driver::Cli(_) => {
                docker(&["stop", "-t", "5", &Self::container_name(ws_id)]).await?;
                Ok(())
            }
        }
    }

    pub async fn destroy(&self, ws_id: &str) -> Result<()> {
        match &self.driver {
            Driver::Mock => Ok(()),
            Driver::Cli(_) => {
                docker(&["rm", "-f", &Self::container_name(ws_id)]).await?;
                Ok(())
            }
        }
    }

    /// Hjelper for normal oppstartsflyt: create + start + vent til klar.
    pub async fn provision(&self, ws_id: &str) -> Result<WorkspaceState> {
        self.create(ws_id).await?;
        self.start(ws_id).await?;
        // Vent på at containeren er oppe og porten er publisert.
        for _ in 0..60 {
            if let WorkspaceState::Running { url } = self.status(ws_id).await? {
                return Ok(WorkspaceState::Running { url });
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        bail!("workspace ble ikke klar innen tidsfristen")
    }
}

impl CliDriver {
    async fn create(&self, name: &str) -> Result<()> {
        // Sikkerhetskrav fra handover: aldri Docker socket, aldri --privileged,
        // aldri host filesystem, aldri host network, aldri RTX 5080.
        // Argumentlisten er fast og bygges kun fra server-side konfig.
        let port_spec = "0.0.0.0::3000"; // tilfeldig vertsport -> KasmVNC web
        let args = [
            "create",
            "--name",
            name,
            "--label",
            "app=skjermsamling",
            "--security-opt",
            "no-new-privileges:true",
            "--cap-drop",
            "ALL",
            "--shm-size",
            &self.shm_size,
            "--memory",
            &self.memory,
            "--cpus",
            &self.cpus,
            "-p",
            port_spec,
            "-e",
            "TITLE=Skjermsamling",
            &self.image,
        ];
        docker(&args).await?;
        Ok(())
    }

    async fn status(&self, name: &str) -> Result<WorkspaceState> {
        let state = match docker(&["inspect", "-f", "{{.State.Status}}", name]).await {
            Ok(s) => s.trim().to_string(),
            Err(_) => return Ok(WorkspaceState::None),
        };
        match state.as_str() {
            "running" => {
                let port_out = docker(&["port", name, "3000/tcp"]).await?;
                // Format: "0.0.0.0:49155" (ev. flere linjer med IPv6)
                let port = port_out
                    .lines()
                    .filter_map(|l| l.rsplit(':').next())
                    .next()
                    .context("fant ikke publisert port")?
                    .trim()
                    .to_string();
                let host = if self.public_host.is_empty() {
                    "{host}".to_string()
                } else {
                    self.public_host.clone()
                };
                Ok(WorkspaceState::Running {
                    url: format!("http://{host}:{port}/"),
                })
            }
            "created" | "restarting" => Ok(WorkspaceState::Starting),
            "exited" | "dead" => Ok(WorkspaceState::Error {
                message: format!("container er i tilstand '{state}'"),
            }),
            _ => Ok(WorkspaceState::Starting),
        }
    }
}

async fn docker(args: &[&str]) -> Result<String> {
    let out = Command::new("docker")
        .args(args)
        .output()
        .await
        .context("klarte ikke å kjøre docker-CLI")?;
    if !out.status.success() {
        return Err(anyhow!(
            "docker {} feilet: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
