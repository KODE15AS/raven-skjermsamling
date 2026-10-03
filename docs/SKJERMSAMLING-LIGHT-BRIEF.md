# Skjermsamling Light – brief for oppstart

Dato: 3. oktober 2026. Dette dokumentet er startpunktet for en ny chat som
skal bygge «Skjermsamling Light». Les også `AGENTS.md` (husregler),
`README.md` (dagens arkitektur) og `docs/KABLET-OPPKOBLING.md`
(skjermoppsettet på Raven) før bygging.

## Konseptet (essensen av Skjermsamling)

To deltagere jobber i samme prosjekt, hver fra sin PC. Hver deltager eier én
workspace (Chrome) som kjører på Raven. Raven sin 70"-skjerm (fast HDMI,
ordinær skjerm) viser begge workspaces side om side, 50/50. På egen PC ser
hver deltager sin egen workspace i fullskjerm — og kan se, peke i og **ta
over** kollegaens workspace. Hensikten er interaktivitet og deltagelse fra
begge inn i samme prosjekt, med veggen som felles referanse.

## Hva Light er — og ikke er

Light er en **nedstrippet produktopplevelse**, ikke en teknisk nyvinning:

| Beholdes (essensen) | Fjernes fra fulle Skjermsamling |
| --- | --- |
| To workspaces på Raven (session-controller, faste Docker-ops) | Dynamisk antall deltagere (fast: 2) |
| «Min skjerm» i fullskjerm på egen PC | Observer-rollen |
| Se / peke (ghost-cursors) / ta over kollegaens workspace | Roster-stripe, chips, «Alle»-visning |
| Vegg på 70" (read-only, 50/50 fast layout) | Minimize/restore |
| Reconnect uten ny session | Navnebasert join med fargetildeling (erstattes av to faste plasser) |

## Beslutninger som er tatt (etter grilling)

1. **Gjenbruk, ikke nybygg.** Light bygges som avlegger av dette repoet og
   gjenbruker backend-komponentene som kostet mest å få riktige:
   session-controlleren (faste ops, capabilities-listen, HTTP-readiness-
   probe), WS-protokollen, Kasm-imagevalget og Dockerfile-oppsettet
   (bookworm-pinning). Å bygge fra null gjentar betalte lærepenger
   (glibc-mismatch, Selkies/HTTPS, 502-race, s6-capabilities — se
   README «Notater til neste handover»).
   Anbefalt form: eget repo `skjermsamling-light`, seedet fra dette repoet
   og strippet — alternativt en «light»-profil i samme repo hvis
   vedlikehold av to repoer blir tyngre enn gevinsten. Avgjøres i
   byggechatten.
2. **To faste plasser** («Venstre» og «Høyre»), ikke navnebasert join.
   Lobby = to store knapper: «Ta plass venstre» / «Ta plass høyre», valgfritt
   navnefelt. En plass som er tatt, er låst til den reconnecter eller
   forlater. Ingen andre valg i UI-et.
3. **Workspace = Chrome** som i dag. `Workspace`-abstraksjonen beholdes slik
   at andre apptyper kan komme senere; Light generaliserer ikke dette nå.
4. **Overtagelse skjer fra egen PC** (dobbeltklikk på kollegaens visning),
   aldri på veggen. Veggen er alltid read-only. Én ekstern controller
   (eieren har alltid samtidig kontroll) — som i dag.
5. **Veggen: fast HDMI, ordinær skjerm.** Ingen hotplug, ingen kabelflytting,
   ingen TV-nettverksstyring (fase 2-planen i HANDOVER-1-TIL-2 legges på is
   for Light). Kiosk-stacken fra `wall/` gjenbrukes: watcher + wmctrl-
   fullskjerm, `WALL_POSITION` satt fast siden 70"-en og 24"-en står i hver
   sin utgang permanent. NVIDIA-KMS-av og Xorg-oppsettet på Raven beholdes.
6. **Flyktige sessions i MVP** (som i dag: ingenting lagres, workspace
   slettes ved leave/timeout). Se åpent spørsmål 1 om persistens.
7. **KODE15 designprofil** gjenbrukes (`frontend/src/app.css`): lys lobby,
   mørk stage, Bebas Neue + Open Sans, eksisterende fargepalett. Venstre/
   høyre plass får hver sin faste farge fra paletten.

## Åpne spørsmål (med default hvis ubesvart)

1. **Persistens:** Skal plassene være prosjekt-varige (Chrome-profil på
   volum, overlever dager/uker) i stedet for flyktige? Dette er den ene
   beslutningen som endrer både arkitektur (volumer, opprydding) og
   sikkerhetsprinsippet «ephemeral sessions» fra handover 0.
   *Default: nei — flyktig i MVP, persistens som eget, senere steg.*
2. **Eget repo eller light-profil i dette repoet?**
   *Default: eget repo, seedet herfra.*
3. **Trenger veggen en statuslinje** (hvem sitter på plassene, hvem
   kontrollerer hvem), eller ren 50/50 uten chrome?
   *Default: minimal navnelinje per halvdel + «X kontrollerer»-badge,
   gjenbrukt fra dagens Tile.*

## Arkitektur (gjenbruk fra dette repoet)

- **Backend (Rust/axum):** som i dag, forenklet hub: to faste slots i stedet
  for dynamisk deltagerliste. WS-meldinger gjenbrukes (join→claim, cursor
  med tile-referanse, control/release, welcome/roster/cursor). `?watch=1`
  beholdes for veggen.
- **Session-controller:** uendret gjenbruk — faste ops, `:kasm`-image,
  capabilities-liste, HTTP-probe med status<500, `host.docker.internal`
  + 127.0.0.1-fallback, ready-timeout.
- **Frontend (Svelte 5):** tre visninger: Lobby (to knapper), Min plass
  (egen workspace fullskjerm + «se kollega»-modus med overtagelse),
  Vegg (50/50, read-only). Tile/Cursors-komponentene gjenbrukes.
- **Drift:** docker-compose som i dag (egen port hvis Light skal kjøre
  parallelt med fulle Skjermsamling på 8015 — foreslått: 8016).
  `MAX_ACTIVE_WORKSPACES=2` hardkodes i Light.

## Kjente forutsetninger og risikoer (lærdommer fra fase 1)

- **Nettverk (uløst):** PC-er på WiFi når ikke Raven på LAN-IP (10.5.0.22)
  — kun Tailscale eller kablet nett fungerer. Må fikses i ruteren hvis
  deltagerne skal sitte på WiFi. Gjelder Light nøyaktig som i dag.
- **Første join etter deploy** laster workspace-imaget (~4–5 GB):
  pre-pull `lscr.io/linuxserver/chromium:kasm`.
- **Veggen:** kiosk-skriptene må kjøres i desktop-sessionen på Raven (aldri
  SSH), autostart-fila må ligge i `~/.config/autostart/` (mappe!), og
  `wmctrl` må være installert. Alt dokumentert i `docs/KABLET-OPPKOBLING.md`.
- **Svart/feilende tile rett etter oppstart:** reload trigger nytt forsøk
  (eksisterende oppførsel); behold denne.

## Foreslåtte milepæler for byggechatten

1. **M1 – Skjelett:** repo/profil opprettet, backend med to faste slots,
   mock-driver, lobby med to knapper, «Min plass»-visning. Testbart uten
   Docker (`WORKSPACE_DRIVER=mock`).
2. **M2 – Samhandling:** se kollega, ghost-cursors, overtagelse med badge,
   Esc/avslutt-knapp. Protokolltest à la fase 1 (`ws-test`).
3. **M3 – Veggen:** /wall 50/50 read-only + kiosk-oppsett på Raven
   (gjenbruk `wall/`), `WALL_POSITION` for fast to-skjerms-oppsett.
4. **M4 – Drift på Raven:** ekte Docker-driver, to deltagere på hver sin PC,
   full gjennomkjøring av testprosedyren (join begge plasser → veggen viser
   50/50 → peking → overtagelse → reconnect → leave rydder opp).
