# Kablet oppkobling – Skjermsamling på Raven

Kort og presis fasit for hvordan skjermene skal kobles til Raven, hvorfor,
og hvordan oppsettet verifiseres. Basert på feilsøking utført 11.–12. sep 2026.

## Grunnregel: all skjerm på Intel, aldri på RTX

| Skjerm | Kobles til | Kontakt (DRM-navn) |
| --- | --- | --- |
| 70" Samsung (veggen) | Hovedkortets **HDMI** | `card0-HDMI-A-1` |
| 24" arbeidsskjerm | Hovedkortets **DisplayPort** (ev. USB-C/DP) | `card0-DP-1` / `card0-DP-2` |
| — | ~~RTX 5080 (HDMI/DP)~~ **aldri** | `card1-*` |

- GUI kjører på Intel UHD 770 (hovedkortets utganger). RTX 5080 er reservert
  CUDA/AI og har skjermstyring (KMS) avslått — portene på RTX-kortet er
  bevisst døde for skjermbruk.
- Kiosken på 70"-en gjenkjenner skjermen på at EDID inneholder `SAMSUNG`
  (konfigurerbart via `WALL_MATCH`).

## Begge skjermer skal stå i permanent

**Ikke flytt én kabel mellom skjermene.** Når den eneste tilkoblede skjermen
trekkes ut, står maskinen et øyeblikk med null skjermer — det krasjer
GNOME/Xorg («Oh no! Something has gone wrong», sessionen dør og alt må
startes på nytt). Dette er observert og reprodusert på Raven.

Riktig drift:

- 24" fast i DP, 70" fast i HDMI. Ingen kabler røres i hverdagen.
- 70"-en slås av/på med fjernkontrollen. Watcheren starter/stopper kiosken
  automatisk etter om skjermen er synlig og tjenesten kjører.
- Med to skjermer tilkoblet samtidig: sett `WALL_POSITION` (X,Y fra
  `xrandr --query`) i autostart-oppsettet slik at kiosken alltid legger seg
  på 70"-en.

Har hovedkortet mot formodning ingen ledig DP/USB-C-utgang, finnes en plan B:
kernel-parameteren `video=HDMI-A-1:D` tvinger HDMI-kontakten til å regnes som
alltid tilkoblet, slik at null-skjerm-øyeblikket aldri oppstår. Ikke satt opp
per i dag — spør i neste økt om behovet melder seg.

## Systemkonfigurasjon som er gjort på Raven

1. **NVIDIA-KMS av** (hindrer GNOME i å røre RTX-en ved skjermendringer;
   krasjet med «Failed to get memory pages» før dette):

   ```bash
   # /etc/modprobe.d/skjermsamling-nvidia-kms-off.conf
   options nvidia-drm modeset=0 fbdev=0
   ```

   Aktivert med `sudo update-initramfs -u` + omstart.
   Verifiser: `sudo cat /sys/module/nvidia_drm/parameters/modeset` → `N`.
   CUDA påvirkes ikke (`nvidia-smi` skal svare normalt).

2. **Desktop-session: GNOME på Xorg** (`loginctl show-session <seat0-ID> -p Type`
   → `Type=x11`).

3. **systemd-coredump installert** — neste ev. krasj gir full stack via
   `coredumpctl list`.

4. **Wall-watcher installert** (fra dette repoet, se README «70"-veggen»):
   - Skript: `/opt/skjermsamling/skjermsamling-wall-watcher.sh`
   - Autostart: `~/.config/autostart/skjermsamling-wall-watcher.desktop`
     (NB: `~/.config/autostart` må være en **mappe** — en tidligere feil
     hadde lagret den som fil, da kjører ingenting ved innlogging)
   - `wmctrl` installert (tvinger kiosken til fullskjerm; Chromium ignorerer
     egne fullskjerm-flagg på dette oppsettet)
   - Watcheren virker kun i desktop-sessionen. Startes den over SSH,
     avslutter den med feilmelding — det er med vilje.

## Verifisering (kjør på Raven etter omstart/innlogging)

```bash
sudo cat /sys/module/nvidia_drm/parameters/modeset   # N
curl -s http://localhost:8015/healthz                 # ok
pgrep -af skjermsamling-wall-watcher                  # én prosess
tail -5 /tmp/wall-watcher.log                         # «Tvang kiosk-vinduet til fullskjerm (wmctrl)» når 70" er på
```

Med 70"-en påslått skal `/wall` stå i fullskjerm uten at noen rører Raven.

## Kjente feil og botemidler

| Symptom | Årsak | Botemiddel |
| --- | --- | --- |
| «Oh no! Something has gone wrong» ved skjermbytte | Kabelflytting ga null-skjerm-øyeblikk | Begge skjermer fast tilkoblet (denne fasiten); logg inn igjen — alt autostarter |
| Kiosk i vindu/halvskjerm | `wmctrl` mangler | `sudo apt install -y wmctrl` |
| Kiosk starter aldri, loggen spinner med «starter kiosk» | Watcher startet over SSH | Start fra terminal på Ravens desktop (eller logg ut/inn) |
| Watcher starter ikke ved innlogging | Autostart-fila mangler / `autostart` er en fil | `mkdir -p ~/.config/autostart && cp wall/skjermsamling-wall-watcher.desktop ~/.config/autostart/` |
| KasmVNC-feil («lastActiveAt») på veggen etter omstart | Stale stream fra før omstart | Ufarlig; frisk kiosk-profil rydder ved neste start |
