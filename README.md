# Skjermsamling

Én generell tjeneste på **Raven** som gir personer i samme fysiske rom en felles,
interaktiv arbeidsflate på 70" 4K-skjermen. Alle deltagere er likeverdige –
ingen host, moderator eller manager.

Hver deltager får sin egen fjernstyrte Chrome-instans som fysisk kjører på
Raven, og 70"-skjermen viser alle aktive workspaces samlet. Opplevelsen skal
være **multiplayer web**, ikke tradisjonell remote desktop.

## Arkitektur

```
┌─────────────────────────────── Raven ───────────────────────────────┐
│                                                                     │
│  skjermsamling (container, port 8015)                               │
│  ├─ Rust-backend (axum): WebSocket presence, SQLite,                │
│  │  session-controller (create/start/stop/status/destroy)           │
│  └─ Svelte-frontend: / (lobby), /samling (tiles), /wall (70")       │
│                                                                     │
│  skjermsamling-ws-<id> … (dynamiske workspace-containere)           │
│  └─ Chromium + KasmVNC web-stream, én per deltager, ephemeral       │
│                                                                     │
│  UHD 770 → 70"-skjerm (Chromium kiosk på /wall)                     │
│  RTX 5080 → reservert CUDA/AI/LLM (brukes ALDRI av Skjermsamling)   │
└─────────────────────────────────────────────────────────────────────┘
```

- **`Workspace`** er den interne abstraksjonen. Eneste type i MVP er Chrome,
  men modellen er laget for senere typer (Linux desktop, terminal, VS Code,
  AI-agent, …) uten å endre produktmodellen.
- **Session-controlleren** i backend er eneste komponent som får starte/stoppe
  workspaces, og støtter kun de faste operasjonene `create`, `start`, `stop`,
  `status`, `destroy`. Frontend kan aldri sende Docker-parametere.
- Workspace-containerne får aldri docker-socket, `--privileged`,
  host-filsystem, host-nettverk eller RTX 5080 (`--cap-drop ALL`,
  `no-new-privileges`, minne-/CPU-grenser).

## Kom i gang på Raven

```bash
git clone https://github.com/KODE15AS/raven-skjermsamling
cd raven-skjermsamling
docker compose up -d --build
```

Åpne deretter `http://raven:8015/` (kun godkjent lokalnett/Tailscale – ingen
login i MVP). Bruker skriver navn, velger **Deltager** eller **Observer**, og
trykker **Gå til Skjermsamling**.

### 70"-veggen

`/wall` er alltid read-only: den joiner aldri sessionen og sender aldri
tastatur- eller mus-input (egen `watch`-modus i WebSocket-protokollen).

**Automatisk kiosk (anbefalt):** `wall/skjermsamling-wall-watcher.sh` kjører
på Ravens lokale Ubuntu-desktop og starter Chromium i kiosk-modus på `/wall`
automatisk når 70"-skjermen kobles til (gjenkjennes på at EDID inneholder
`SAMSUNG`), og lukker kiosken igjen når den kobles fra. Skriptet sjekker også
at Skjermsamling-tjenesten faktisk kjører (via `/healthz`) før det gjør noe —
er containeren nede startes ingen kiosk, og en åpen kiosk lukkes. Den vanlige
24"-skjermen trigger ingenting.

Installasjon (én gang, på Raven):

```bash
sudo apt install -y wmctrl   # brukes til å tvinge fullskjerm (Wayland ignorerer Chromiums egne flagg)
sudo mkdir -p /opt/skjermsamling
sudo cp wall/skjermsamling-wall-watcher.sh /opt/skjermsamling/
cp wall/skjermsamling-wall-watcher.desktop ~/.config/autostart/
# Start uten å logge ut/inn:
/opt/skjermsamling/skjermsamling-wall-watcher.sh &
```

Verifiser hva skjermene identifiserer seg som (EDID) hvis matchen må justeres:

```bash
for d in /sys/class/drm/card*-*/; do
  echo "$d: $(cat $d/status 2>/dev/null)"
  strings "$d/edid" 2>/dev/null | head -3
done
```

Miljøvariabler: `WALL_MATCH` (default `SAMSUNG`), `WALL_URL` (default
`http://localhost:8015/wall`), `WALL_HEALTH_URL` (default
`http://localhost:8015/healthz`), `WALL_BROWSER`, `WALL_POLL_SECS` (default 3)
og `WALL_POSITION` («X,Y», styrer hvilken skjerm kiosken havner på hvis både
24" og 70" er tilkoblet samtidig; finn posisjonen med `xrandr --query`).

**Manuelt alternativ:**

```bash
chromium --kiosk --noerrdialogs --disable-session-crashed-bubble \
  http://localhost:8015/wall
```

## Funksjoner i MVP

| Funksjon | Hvordan |
| --- | --- |
| Join uten konto | Kun navn + rolle; automatisk unik farge (stabil per navn via SQLite) |
| Live presence | WebSocket-broadcast ved join/leave/disconnect/reconnect, minimize/restore, workspace start/stopp og kontroll-endringer |
| Egen Chrome på Raven | Ephemeral container per deltager; slettes ved leave eller når disconnect-timeout utløper (`WORKSPACE_TIMEOUT_SECS`) |
| Tilbake uten ny session | Session-token i localStorage; reconnect innen timeout gjenbruker workspacen |
| Tiles på 70" | Responsivt grid som tilpasser seg antall aktive tiles (1/2/2×2/3×2/3×3) |
| «Min skjerm» / «Alle» | `/samling` åpner i «Min skjerm» (kun egen workspace – samlingen ser man på 70"-veggen). Klikk på en deltager-chip for å se/peke på/ta over dennes workspace, eller bytt til «Alle» for grid med alt |
| Minimize/restore | Egen tile kan skjules fra veggen (navnefane i «Alle»-visningen); kun eieren kan vise den igjen |
| Demokratisk kontroll | Dobbeltklikk en annen tile → aktiver kontroll. Grønn eierramme beholdes, kontrollerens farge vises som ytre ramme + «X kontrollerer». Esc eller dobbeltklikk avslutter. Kun én ekstern controller per workspace |
| Fargede ghost-cursors | HTML-overlay (ikke OS-pekere) med navn, tile-relative x/y via WebSocket (~30 Hz) – pekeren treffer riktig tile uansett hvilken visning mottakeren ser |
| Observer | Navn/farge og presence, ser alle workspaces, får aldri egen Chrome og sender aldri OS-input |

## Konfigurasjon

| Variabel | Default | Beskrivelse |
| --- | --- | --- |
| `MAX_ACTIVE_WORKSPACES` | `4` | Maks samtidige workspaces. Start konservativt, benchmark Raven før økning |
| `WORKSPACE_IMAGE` | `lscr.io/linuxserver/chromium:kasm` | Image for Chrome-workspaces (se merknad om Selkies/HTTPS under) |
| `WORKSPACE_MEMORY` / `WORKSPACE_CPUS` / `WORKSPACE_SHM_SIZE` | `3g` / `2` / `1g` | Ressursgrenser per workspace |
| `WORKSPACE_TIMEOUT_SECS` | `120` | Tid fra disconnect til workspacen stoppes og slettes |
| `WORKSPACE_READY_TIMEOUT_SECS` | `180` | Maks ventetid på at streamen svarer før oppstart regnes som feilet |
| `WORKSPACE_PROBE_HOST` | `host.docker.internal` | Vertsnavn controlleren bruker for readiness-probe (faller tilbake til 127.0.0.1) |
| `WORKSPACE_PUBLIC_HOST` | *(tom)* | Vertsnavn klientene når workspacene på; tom = samme host som nettleseren bruker |
| `WORKSPACE_DRIVER` | `docker` | `mock` for utvikling uten Docker |
| `BIND` | `0.0.0.0:8015` | Backendens lytteadresse |
| `DB_PATH` | `skjermsamling.db` | SQLite-fil (fargetildelinger + hendelseslogg) |
| `STATIC_DIR` | `../frontend/dist` | Katalog med bygget frontend |

## Utvikling (uten Docker)

```bash
# Backend med mock-workspaces
cd backend
WORKSPACE_DRIVER=mock cargo run

# Frontend med hot reload (proxyer /ws til backend)
cd frontend
npm install
npm run dev
```

Mock-driveren gir hver deltager en enkel interaktiv placeholder-side i stedet
for en ekte Chromium-container, slik at hele presence-/kontroll-/cursor-flyten
kan testes på en vanlig utviklingsmaskin.

## Sikkerhet og drift

- MVP er kun for godkjent lokalnett/Tailscale. Ikke eksponer port 8015 eller
  workspace-portene mot internett.
- Kun `skjermsamling`-containeren har docker-socket (den er
  session-controlleren). Workspace-containere startes med fast argumentliste
  bygget server-side: `--cap-drop ALL`, `no-new-privileges`, minne/CPU-grenser,
  aldri GPU.
- Sessions er ephemeral: ingen brukerprofiler; workspaces slettes ved leave
  eller timeout. Brukeren logger selv inn på ønskede websider inne i sin
  Chrome-session.
- Workspace-containerne kjører med `--cap-drop ALL` pluss kun de fem
  capabilities linuxserver-imagene faktisk trenger (CHOWN, SETUID, SETGID,
  FOWNER, DAC_OVERRIDE) — uten disse crash-looper s6/nginx inne i containeren.

## Notater til neste handover

- **Selkies/HTTPS**: `linuxserver/chromium:latest` byttet til Selkies
  (juni 2025), som krever secure context (HTTPS med gyldig sertifikat) og
  viser «This application requires a secure connection» over ren HTTP.
  MVP bruker derfor den frosne Kasm-branchen (`:kasm`, deprecated juli 2026)
  som fungerer over HTTP på lukket nett. Riktig oppgradering senere:
  TLS-terminering foran hele tjenesten (f.eks. `tailscale serve` eller
  Caddy/SWAG med gyldig sertifikat) + proxy av workspace-portene gjennom
  samme origin, og deretter bytte tilbake til Selkies-imaget.
- Første join etter deploy laster ned workspace-imaget (~4–5 GB); kjør
  `docker pull` av `WORKSPACE_IMAGE` på forhånd for å unngå lang ventetid.
- Oppstartsfeil vises nå med tydelig melding på tilen og reload av siden
  trigger nytt forsøk; automatisk retry med backoff kan vurderes senere.
- Raven svarte ikke på lokalnett-IP (kun Tailscale) under første test —
  sjekk brannmur/subnett hvis LAN-tilgang ønskes.
