# Handover 1 → 2: Skjermsamling over WiFi (Samsung-TV som nettverksskjerm)

Dato: 12. september 2026. Forrige handover (0 → 1, «Skjermsamling som raven
konferanserom tjeneste») er med dette **lukket — MVP levert og verifisert i
drift**. Dette dokumentet er startpunktet for neste fase.

## Sluttstatus for fase 0 → 1 (lukket)

Levert og testet på Raven med ekte deltagere (PR #1–#12):

- Rust-backend (axum/WebSocket/SQLite) + Svelte-frontend + session-controller
  med kun faste Docker-operasjoner. `MAX_ACTIVE_WORKSPACES=4`.
- Join med kun navn (Deltager/Observer), unike farger, live presence,
  reconnect innen timeout gjenbruker workspace.
- «Min skjerm» som default på egen PC; chip-klikk for å se/peke/ta over
  andres workspace; «Alle»-visning som grid; `/wall` (read-only) for veggen.
- Tile-relative ghost-cursors, demokratisk kontroll (én ekstern controller),
  minimize/restore.
- Wall-kiosk på Ravens HDMI: watcher som starter/stopper Chromium-kiosk
  automatisk etter EDID-deteksjon (SAMSUNG) + tjeneste-helsesjekk, med
  fullskjerm tvunget via wmctrl. Se `docs/KABLET-OPPKOBLING.md`.
- Systemfikser på Raven: NVIDIA-KMS av (RTX 5080 kun CUDA), GNOME på Xorg,
  systemd-coredump, wmctrl.

Kjent, dokumentert begrensning: kabelflytting mellom 24" og 70" gir et
null-skjerm-øyeblikk som krasjer GNOME (SIGSEGV i
`meta_monitor_get_edid_checksum_md5()`, kjent mutter 46-svakhet — ikke
forårsaket av Skjermsamling). Dette er hovedmotivasjonen for fase 2.

## Mål for fase 1 → 2

**70"-TV-en skal bli en ren nettverksskjerm.** Ingen HDMI-kabel til Raven,
ingenting installeres på TV-en. Raven vekker TV-en over nettet og åpner
`/wall` i TV-ens innebygde nettleser. 24"-skjermen blir stående alene fast på
Raven — null-skjerm-problemet dør av seg selv.

## Beslutninger som er tatt (diskutert og avklart)

1. **Bibliotek:** Python `samsungtvws` (repo: xchwarze/samsung-tv-ws-api).
   Har eksplisitt Wake-on-LAN, fjernstyring og `open_browser(url)`.
2. **Arkitektur:** Permanent intern sidecar-container i Docker
   («wall-controller»), etter samme prinsipp som session-controlleren:
   **kun faste operasjoner** — `wake`, `open_wall`, `status`. Aldri et
   generelt «send hva som helst til TV-en»-API, aldri eksponert utenfor
   Docker-nettet. Rust-backenden kaller den internt.
3. **Vekking, to caser:**
   - Case 1: TV avslått → vekkes automatisk når første deltager joiner.
   - Case 2: TV i bruk (annen kilde) → `open_browser` sendes uansett;
     Skjermsamling tar over.
   - Ingen faste ventetider: retry-løkke (magic packet + API-forsøk) til
     port 8002 svarer.
4. **Token/konfig:** TV-ens IP reserveres i DHCP, MAC lagres i konfig,
   API-token persisteres i Docker-volum (unngår ny godkjenning på TV etter
   redeploy).
5. **Milepæl 0 er porten:** Ingen integrasjon bygges før et frittstående
   testskript har bestått ende-til-ende på den faktiske TV-en (se under).

## TV-en (verifisert mot dokumentasjon)

- Modell: **TU70DU7105KXXC** — 2024 DU7100-serien (nordisk), Tizen OS.
- Bruksanvisningen bekrefter «Power On with Mobile» og «IP Remote» under
  Innstillinger → Tilkobling → Nettverk → Ekspertinnstillinger. Begge må PÅ.
- Sett Device Connection Manager / Access Notification til «First Time Only»
  hvis mulig, så første godkjenning på TV-en blir den eneste.
- Første WebSocket-tilkobling (wss, port 8002) viser godkjenningsdialog på
  TV-en; token lagres deretter.
- TV-en står på WiFi (kablet nett er ikke mulig i rommet).

## Kritisk sti: nettverket må løses FØRST

Observert i fase 1: enheter på WiFi når IKKE Raven på lokalnett-IP
(10.5.0.22) — kun Tailscale fungerte. TV-en kan ikke kjøre Tailscale.
Før milepæl 0 må derfor ruteren/brannmuren fikses slik at:

1. TV (WiFi) når Raven: port 8015 **og** workspace-portene (dynamiske
   host-porter fra Docker).
2. Raven når TV: port 8002 (WebSocket-styring).
3. WoL magic packet når TV-en — broadcast krysser ikke subnett, så TV og
   Raven bør på samme L2/subnett. Prosjektdokumentasjonen til
   samsung-tv-ws-api advarer eksplisitt mot WebSocket på tvers av
   subnett/VLAN.

Sjekk om WiFi-et er et gjestenett/eget VLAN med klientisolasjon.

## Milepæl 0: ende-til-ende-test (før all integrasjon)

Frittstående skript (kjøres fra Raven): magic packet → poll til 8002 svarer
→ `open_browser("http://10.5.0.22:8015/wall")`. Akseptkriterier:

- Vekking fungerer fra: 30 sek av, 1 time av, og etter en natt i standby.
- Case 2: `open_browser` tar over når TV-en viser annen kilde.
- **Ytelse (største risiko):** `/wall` med minst 2 aktive deltager-streams,
  flytende nok til møtebruk, i 30+ minutter — uten at nettleseren dør,
  viser UI-chrome, dimmer eller går i skjermsparer. DU7105 har en beskjeden
  SoC; KasmVNC-streams i canvas er tungt. Stryker TV-nettleseren her,
  beholdes HDMI-kiosken som fasit (den virker og er dokumentert).

## Opprydding når TV-løsningen er bevist (IKKE før)

Fjernes fra Raven: `~/.config/autostart/skjermsamling-wall-watcher.desktop`,
kjørende watcher, `/opt/skjermsamling/`, HDMI-kabelen til 70"-en.
Beholdes: NVIDIA-KMS-av (spec: RTX kun CUDA), systemd-coredump, wmctrl,
chromium. I repoet beholdes `wall/` + `docs/KABLET-OPPKOBLING.md` som
dokumentert fallback.

## Annet utestående

- UEFI dbx-firmwareoppdatering (Firmware Updater) tas i neste
  vedlikeholdsvindu med omstart.
- WiFi→LAN-blokkeringen (punktet over) gjelder også gjester som skal delta
  fra WiFi — samme fiks løser begge.
- Vurder fast portrange for workspaces i session-controlleren hvis
  brannmuren krever eksplisitte regler (i dag: tilfeldige host-porter).

## Nyttig fakta

- Raven: LAN 10.5.0.22, Tailscale 100.65.19.39, app-port 8015.
  Bruker `cadify`, repo i `~/raven-skjermsamling`.
- Tjenesten: `docker compose up -d --build` i repo-rot. Helse: `/healthz`.
- Kiosk-logg (fase 1-løsning): `/tmp/wall-watcher.log`.
- Krasjdiagnostikk: `coredumpctl list`.
