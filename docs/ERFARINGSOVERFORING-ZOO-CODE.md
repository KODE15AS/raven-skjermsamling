# Erfaringsoverføring: Skjermsamling → ny app for zoo.code

Dato: 3. oktober 2026. Dette dokumentet overfører konsept og lærdom fra
Skjermsamling (dette repoet) til en **helt ny applikasjon i et helt nytt
container/repo-oppsett**. Det skal IKKE bygges noe i raven-skjermsamling.
Ta dokumentet med inn i oppstartschatten for det nye repoet.

## Konseptet som videreføres (verifisert i drift, fungerte svært bra)

To deltagere jobber i samme prosjekt fra hver sin PC (via Tailscale).
Hver deltager eier én workspace som kjører på Raven. Raven sin 70"-skjerm
(fast HDMI, ordinær skjerm) viser begge workspaces side om side 50/50 og er
alltid read-only — Ravens tastatur/mus brukes aldri. På egen PC ser hver
deltager sin egen workspace i fullskjerm, og kan se, peke i (fargede
ghost-cursors med navn) og **ta over** kollegaens workspace. Overtagelsen
(dobbeltklikk → ytre ramme i egen farge + «X kontrollerer»-badge → avslutt
med knapp/Esc, kun én ekstern controller om gangen, eieren har alltid
samtidig kontroll) var det som fungerte aller best — videreføres uendret.

## Hva som er nytt i zoo.code-versjonen (avklarte beslutninger)

1. **Helt ny app, nytt repo, ny container-setup.** Kun lærdommen herfra tas
   med; koden skrives nytt og tilpasses zoo.code.
2. **Workspace-innholdet er zoo.code**, ikke Chrome: hver deltagers flate
   viser zoo.code sitt workspace og nettsiden som bygges i zoo.code.
3. **Prosjekter er persistente.** Ett prosjekt = ett eget container/repo-sett
   = én økt som kan gjenopptas. Nytt prosjekt → nytt sett startes. (Motsatt
   av Skjermsamlings flyktige sessions — persistens er et bevisst valg her.)
4. **Interaksjon kun fra deltagernes egne PC-er.** Veggen viser aktiviteten;
   overtagelse av kollega skjer fra egen PC, som i Skjermsamling.
5. **Nettverk: to deltagere på Tailscale (WiFi).** Standardiser alt på
   Tailscale-adresser — det omgår den kjente blokkeringen WiFi→LAN hos
   KODE15 (Raven på LAN-IP er uoppnåelig fra WiFi; Tailscale fungerer
   stabilt og er testet).
6. **Fast skjerm på Raven:** 70"-en står permanent i hovedkortets HDMI som
   ordinær skjerm. Ingen hotplug, ingen kabelflytting, ingen TV-styring
   over nett.

## Tekniske lærdommer — betalt én gang, skal ikke betales igjen

### A. GUI-streaming fra containere (hvis den nye appen trenger det)

- `linuxserver/*:latest` byttet til **Selkies** (jun. 2025) som krever
  HTTPS/secure context — over ren HTTP får man bare en feilmelding. Den
  frosne **`:kasm`**-branchen (KasmVNC) fungerer over HTTP på lukket nett.
  Velg streaming-teknologi bevisst og test HTTP/HTTPS-kravet først.
- `--cap-drop ALL` knekker linuxserver-images (s6/nginx crash-looper med
  «Operation not permitted»). Minimumssettet som trengs: CHOWN, SETUID,
  SETGID, FOWNER, DAC_OVERRIDE.
- **TCP-probe er ikke nok** for «klar»-sjekk: nginx svarer på TCP før
  upstream er klar → 502 i iframen (som aldri selv retryer). Bruk HTTP-probe
  som krever status < 500, via `host.docker.internal` (extra_hosts:
  host-gateway) med 127.0.0.1-fallback. Ha ready-timeout med tydelig
  feilmelding på flaten, og la reload trigge nytt forsøk.
- **glibc:** build- og runtime-stage i Dockerfile må bruke samme
  Debian-versjon (rust:slim er trixie-basert; pin f.eks.
  `rust:<ver>-slim-bookworm` mot `debian:bookworm-slim`).
- Store workspace-images (4–5 GB): **pre-pull** før første bruk, ellers
  venter første deltager i mange minutter.

### B. Sikkerhetsprinsipper (gjenbrukes som de er)

- Én controller-komponent er den **eneste** som starter/stopper containere,
  og kun gjennom faste operasjoner (create/start/stop/status/destroy).
  Frontend kan aldri sende vilkårlige container-parametre.
- Workspace-containere får aldri: docker-socket, `--privileged`,
  host-filsystem, host-nettverk eller GPU.
- Tjenesten er kun for Tailscale/godkjent nett. Ingen eksponering mot
  internett.
- På Raven: GUI på Intel UHD 770, **RTX 5080 kun CUDA** — `nvidia-drm
  modeset=0` er satt og skal ikke reverseres.

### C. Presence/WebSocket-protokoll (design som virket)

- **Abonner på broadcast FØR join, og send roster-snapshot direkte til ny
  socket** etter welcome — ellers race der nye tilkoblinger mister
  deltagerlisten.
- **Reconnect uten ny økt:** session-token i localStorage + disconnect-
  timeout på server med generasjonsteller (avbryter utdatert opprydding
  når noen rekker tilbake).
- **Tile-relative cursor-koordinater** (eier-id + normalisert x/y innenfor
  flaten) — pekere treffer riktig uansett hvilken visning mottakeren ser
  (egen fullskjerm, kollega-visning eller 50/50-vegg).
- **Veggen har egen watch-modus** (`?watch=1`): joiner aldri, sender aldri
  input, kan aldri påvirke økten.
- Kontroll: kun én ekstern controller per workspace; take/release
  broadcastes til alle.

### D. Veggen på Raven (gjenbrukbart oppsett, se wall/ og docs/)

- Chromium ignorerer `--kiosk`/`--start-fullscreen` på dette oppsettet:
  bruk `--ozone-platform=x11` + **tving fullskjerm via wmctrl** (match
  vinduet på PID, aldri tittel/klasse), frisk `--user-data-dir` per start.
- Kiosk-skript må kjøres i desktop-sessionen, **aldri over SSH** (legg inn
  DISPLAY-vakt med tydelig feilmelding — ellers spinner det stille).
- `~/.config/autostart/` må være en mappe (klassisk felle: cp lagde fil med
  det navnet → autostart døde stille).
- **Hotplug av skjerm krasjer GNOME** (SIGSEGV i mutter EDID-håndtering,
  Ubuntu 24.04/Xorg) — derfor fast tilkoblet skjerm. Ved to skjermer:
  `WALL_POSITION` (xrandr-koordinat) styrer hvilken skjerm kiosken tar.
- Kiosken bør være **helse-gatet**: sjekk tjenestens `/healthz` før start,
  og lukk kiosken når tjenesten er nede.

### E. Nettverk og URL-er

- Ikke hardkod vertsnavn i frontend: `{host}`-substitusjon (erstatt med
  `location.hostname` i klienten) gjorde at samme deploy virket via både
  LAN og Tailscale.
- WiFi→LAN er blokkert hos KODE15 (uløst i ruteren). Tailscale er
  standarden for den nye appen (beslutning 5) — test alle URL-er derfra.

### F. Design og UX

- KODE15-designprofilen ligger i `frontend/src/app.css` i dette repoet:
  lys lobby, mørk «stage» rundt levende flater, Bebas Neue + Open Sans,
  fargepalett med faste eierfarger. Kopier grunnlaget.
- Eierfarge som permanent ramme + controllerfarge som ytre ramme ga
  umiddelbar lesbarhet på veggen.
- Tydelige feilmeldinger med handling («reload prøver igjen») fremfor
  evig spinner — ble etterspurt eksplisitt under testing.

### G. Prosess (fungerte godt, gjenta)

- `AGENTS.md` i repo-roten fra dag én (oppstartsrutine, prinsipper, norsk).
- Handover-/brief-dokumenter i `docs/` som eneste sannhetskilde mellom
  økter — ikke chat-hukommelse.
- **Mock-driver** (`WORKSPACE_DRIVER=mock`) slik at hele presence/kontroll/
  cursor-flyten kan utvikles og testes uten Docker/GPU-maskin.
- Protokoll-smoketest mot mock (à la 16-punkts ws-test) fanget regresjoner
  raskt. Små, fokuserte PR-er mot main.

## Åpne spørsmål den nye chatten må avklare først

1. **Er zoo.code web-basert eller desktop-GUI?** Avgjør hele
   streaming-laget: er zoo.code en webapp, kan flatene vises direkte
   (iframe/URL) uten KasmVNC-streaming i det hele tatt — mye lettere.
   Er det desktop-GUI, gjenbrukes streaming-lærdommene i A.
2. **Hva vises per deltagerhalvdel:** zoo.code-arbeidsflaten, web-forhånds-
   visningen av siden som bygges, eller begge (og hvordan veksles det)?
3. **Persistens-mekanikken:** hvordan startes/gjenopptas et prosjekt
   (manuell oppstart av prosjektets container/repo-sett, eller en enkel
   prosjektvelger i lobbyen)? Hva er oppryddingsrutinen når et prosjekt
   avsluttes?
4. **Overtagelse i zoo.code:** hvis zoo.code har egen flerbruker-/
   samarbeidsstøtte, kan «ta over» realiseres naturlig der i stedet for
   input-streaming — undersøk før streaming-veien velges.

## Referanser i dette repoet

- `README.md` — arkitektur, konfigtabell, «Notater til neste handover»
- `AGENTS.md` — husregler (mal for den nye appens AGENTS.md)
- `docs/KABLET-OPPKOBLING.md` — skjerm-/kiosk-oppsettet på Raven
- `docs/HANDOVER-1-TIL-2.md` — TV-over-nett-planen (lagt på is)
- `wall/` — kiosk-skriptene (gjenbrukbare for veggen)
- `backend/src/workspace.rs` — controller med faste ops, probe, capabilities
- `backend/src/hub.rs` — presence/kontroll/cursor-protokollen
