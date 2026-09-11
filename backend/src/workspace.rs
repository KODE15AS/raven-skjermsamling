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
    /// Vertsnavn controlleren bruker for readiness-probe mot publiserte
    /// workspace-porter (inne i compose-containeren: host.docker.internal).
    probe_host: String,
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
                image: env_or("WORKSPACE_IMAGE", "lscr.io/linuxserver/chromium:kasm"),
                memory: env_or("WORKSPACE_MEMORY", "3g"),
                cpus: env_or("WORKSPACE_CPUS", "2"),
                shm_size: env_or("WORKSPACE_SHM_SIZE", "1g"),
                public_host: env_or("WORKSPACE_PUBLIC_HOST", ""),
                probe_host: env_or("WORKSPACE_PROBE_HOST", "host.docker.internal"),
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
    /// `status` rapporterer først Running når streamen faktisk svarer på
    /// porten, så tilen aldri får en URL som avviser tilkoblinger.
    pub async fn provision(&self, ws_id: &str) -> Result<WorkspaceState> {
        // Rydd bort ev. etterlatt container med samme navn (f.eks. etter
        // backend-restart eller feilet forrige forsøk) før nytt forsøk.
        let _ = self.destroy(ws_id).await;
        self.create(ws_id).await?;
        self.start(ws_id).await?;
        let timeout_secs: u64 = std::env::var("WORKSPACE_READY_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(180);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
        while tokio::time::Instant::now() < deadline {
            match self.status(ws_id).await? {
                WorkspaceState::Running { url } => return Ok(WorkspaceState::Running { url }),
                WorkspaceState::Error { message } => bail!("workspace feilet: {message}"),
                _ => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        }
        bail!("workspace ble ikke klar innen {timeout_secs} sekunder")
    }
}

impl CliDriver {
    async fn create(&self, name: &str) -> Result<()> {
        // Sikkerhetskrav fra handover: aldri Docker socket, aldri --privileged,
        // aldri host filesystem, aldri host network, aldri RTX 5080.
        // Argumentlisten er fast og bygges kun fra server-side konfig.
        //
        // Alle capabilities droppes, og kun de fem s6-init/nginx i
        // linuxserver-imagene faktisk trenger legges tilbake (uten disse
        // crash-looper tjenestene inne i containeren med
        // "chown: Operation not permitted").
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
            "--cap-add",
            "CHOWN",
            "--cap-add",
            "SETUID",
            "--cap-add",
            "SETGID",
            "--cap-add",
            "FOWNER",
            "--cap-add",
            "DAC_OVERRIDE",
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
                let port: u16 = port_out
                    .lines()
                    .filter_map(|l| l.rsplit(':').next())
                    .next()
                    .context("fant ikke publisert port")?
                    .trim()
                    .parse()
                    .context("ugyldig portnummer")?;
                // Readiness: rapportér først Running når streamen faktisk
                // aksepterer tilkoblinger på den publiserte porten.
                if !self.probe(port).await {
                    return Ok(WorkspaceState::Starting);
                }
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

    /// HTTP-probe mot publisert workspace-port. Inne i compose-containeren
    /// nås vertens porter via host.docker.internal (extra_hosts i compose);
    /// ved kjøring rett på verten faller vi tilbake til 127.0.0.1.
    ///
    /// En ren TCP-connect er ikke nok: nginx i workspace-imaget aksepterer
    /// tilkoblinger før tjenestene bak er klare og svarer da 502. Vi krever
    /// derfor et ekte HTTP-svar med status < 500 før workspacen regnes som
    /// klar, slik at tiles aldri får en URL som viser «502 Bad Gateway».
    async fn probe(&self, port: u16) -> bool {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for host in [self.probe_host.as_str(), "127.0.0.1"] {
            let attempt = tokio::time::timeout(
                Duration::from_millis(1000),
                tokio::net::TcpStream::connect((host, port)),
            );
            let Ok(Ok(mut stream)) = attempt.await else {
                continue;
            };
            let req = b"GET / HTTP/1.0\r\nHost: probe\r\nConnection: close\r\n\r\n";
            if stream.write_all(req).await.is_err() {
                continue;
            }
            let mut buf = [0u8; 64];
            let Ok(Ok(n)) = tokio::time::timeout(
                Duration::from_millis(1500),
                stream.read(&mut buf),
            )
            .await
            else {
                continue;
            };
            // Statuslinje: "HTTP/1.1 200 OK"
            let line = String::from_utf8_lossy(&buf[..n]);
            if let Some(code) = line
                .split_whitespace()
                .nth(1)
                .and_then(|c| c.parse::<u16>().ok())
            {
                if code < 500 {
                    return true;
                }
            }
        }
        false
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
