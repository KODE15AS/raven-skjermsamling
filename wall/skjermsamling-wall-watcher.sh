#!/usr/bin/env bash
# Skjermsamling wall-watcher
#
# Kjøres på Ravens lokale desktop-session. Overvåker DRM-tilkoblingene i
# /sys/class/drm og starter Chromium i kiosk-modus på /wall automatisk når
# en skjerm hvis EDID matcher WALL_MATCH (default: SAMSUNG) kobles til —
# f.eks. 70"-skjermen i konferanserommet. Når skjermen kobles fra, lukkes
# kiosken igjen. Den vanlige 24"-skjermen trigger ingenting.
#
# Skriptet sjekker først at Skjermsamling-tjenesten faktisk kjører (via
# /healthz). Kjører den ikke, hoppes skjermsjekken over og en eventuell
# åpen kiosk lukkes.
#
# Konfigurasjon (miljøvariabler):
#   WALL_MATCH      Tekst som må finnes i skjermens EDID (default "SAMSUNG")
#   WALL_URL        URL som vises (default http://localhost:8015/wall)
#   WALL_HEALTH_URL Helsesjekk-URL for tjenesten
#                   (default http://localhost:8015/healthz)
#   WALL_BROWSER    Nettleser-kommando (default: chromium, ev. fallback)
#   WALL_POLL_SECS  Sjekkintervall i sekunder (default 3)
#   WALL_POSITION   Valgfri "X,Y" vindusposisjon for kiosken, for oppsett der
#                   både 24" og 70" er tilkoblet samtidig på X11 (posisjonen
#                   avgjør hvilken skjerm kiosken havner på; finn den med
#                   `xrandr --query`).

set -u

MATCH="${WALL_MATCH:-SAMSUNG}"
URL="${WALL_URL:-http://localhost:8015/wall}"
HEALTH_URL="${WALL_HEALTH_URL:-http://localhost:8015/healthz}"
POLL="${WALL_POLL_SECS:-3}"
POSITION="${WALL_POSITION:-}"
PROFILE="${XDG_RUNTIME_DIR:-/tmp}/skjermsamling-wall-profile"

log() { echo "[wall-watcher] $(date '+%H:%M:%S') $*"; }

# Finn nettleser: eksplisitt valgt, ellers første tilgjengelige.
find_browser() {
  if [ -n "${WALL_BROWSER:-}" ]; then
    echo "$WALL_BROWSER"
    return
  fi
  for b in chromium chromium-browser google-chrome; do
    if command -v "$b" >/dev/null 2>&1; then
      echo "$b"
      return
    fi
  done
  echo ""
}

BROWSER="$(find_browser)"
if [ -z "$BROWSER" ]; then
  log "FEIL: fant ingen chromium/chrome i PATH. Installer med: sudo snap install chromium"
  exit 1
fi

# Er Skjermsamling-tjenesten (containeren) oppe? Sjekkes via /healthz slik at
# skriptet ikke trenger docker-rettigheter.
service_running() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsS --max-time 2 -o /dev/null "$HEALTH_URL" 2>/dev/null
  else
    wget -q -T 2 -O /dev/null "$HEALTH_URL" 2>/dev/null
  fi
}

# Sjekk om en tilkoblet skjerm matcher (EDID inneholder produsent-/modellnavn
# som ASCII-tekst; Samsung-skjermer inneholder typisk «SAMSUNG»).
wall_screen_connected() {
  local st dir
  for st in /sys/class/drm/card*-*/status; do
    [ -e "$st" ] || continue
    [ "$(cat "$st" 2>/dev/null)" = "connected" ] || continue
    dir="$(dirname "$st")"
    if strings "$dir/edid" 2>/dev/null | grep -qi -- "$MATCH"; then
      return 0
    fi
  done
  return 1
}

kiosk_running() {
  pgrep -f -- "user-data-dir=$PROFILE" >/dev/null 2>&1
}

# Chromiums egne fullskjerm-flagg blir ignorert i enkelte Wayland/snap-oppsett.
# Derfor tvinger vi i tillegg fullskjerm på vindusbehandler-nivå: vent til
# kiosk-vinduet finnes (matchet på PID, så vi aldri rører andre vinduer), og
# sett fullskjerm-state med wmctrl eller xdotool. Krever at én av dem er
# installert (sudo apt install -y wmctrl).
force_fullscreen() {
  local i pid wid
  for i in $(seq 1 30); do
    sleep 1
    pid="$(pgrep -of -- "user-data-dir=$PROFILE" 2>/dev/null)"
    [ -n "$pid" ] || continue
    if command -v wmctrl >/dev/null 2>&1; then
      wid="$(wmctrl -lp 2>/dev/null | awk -v p="$pid" '$3==p {print $1; exit}')"
      [ -n "$wid" ] || continue
      wmctrl -i -r "$wid" -b add,fullscreen 2>/dev/null
      log "Tvang kiosk-vinduet til fullskjerm (wmctrl)"
      return 0
    elif command -v xdotool >/dev/null 2>&1; then
      wid="$(xdotool search --pid "$pid" --onlyvisible 2>/dev/null | head -1)"
      [ -n "$wid" ] || continue
      xdotool key --window "$wid" F11 2>/dev/null
      log "Tvang kiosk-vinduet til fullskjerm (xdotool)"
      return 0
    else
      log "MERK: verken wmctrl eller xdotool er installert – kan ikke tvinge fullskjerm automatisk. Installer med: sudo apt install -y wmctrl"
      return 1
    fi
  done
  log "MERK: fant aldri kiosk-vinduet – fullskjerm ble ikke tvunget"
  return 1
}

start_kiosk() {
  # Chromium på ren Wayland ignorerer --kiosk/--start-fullscreen (kjent bug);
  # --ozone-platform=x11 tvinger XWayland, der kiosk-modus fungerer pålitelig
  # OG gjør vinduet synlig for wmctrl/xdotool.
  # På en ren X11-session er flagget harmløst (x11 brukes uansett).
  local args=(
    --ozone-platform=x11
    --kiosk
    --start-fullscreen
    --noerrdialogs
    --disable-session-crashed-bubble
    --user-data-dir="$PROFILE"
  )
  if [ -n "$POSITION" ]; then
    args+=(--window-position="$POSITION")
  fi
  args+=("$URL")
  # Frisk profil hver gang, så gammel vindusstørrelse aldri gjenbrukes.
  rm -rf "$PROFILE"
  log "70\"-skjerm oppdaget (match: $MATCH) – starter kiosk: $BROWSER"
  nohup "$BROWSER" "${args[@]}" >/dev/null 2>&1 &
  force_fullscreen &
}

stop_kiosk() {
  log "Skjermen er koblet fra eller tjenesten er nede – lukker kiosken"
  pkill -f -- "user-data-dir=$PROFILE" 2>/dev/null
}

log "Starter. Ser etter skjerm med EDID som matcher «$MATCH», intervall ${POLL}s."
log "Skjermsjekk gjøres bare når tjenesten svarer på $HEALTH_URL."

while true; do
  if service_running && wall_screen_connected; then
    if ! kiosk_running; then
      # Gi desktopen et par sekunder til å aktivere den nye skjermen først.
      sleep 2
      start_kiosk
    fi
  else
    # Skjerm koblet fra ELLER tjenesten er nede: lukk kiosken.
    if kiosk_running; then
      stop_kiosk
    fi
  fi
  sleep "$POLL"
done
