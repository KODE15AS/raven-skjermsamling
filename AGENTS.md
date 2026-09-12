# Husregler for agenter i raven-skjermsamling

Disse reglene gjelder alle AI-agenter som jobber i dette repoet, uansett om
de kjører lokalt, via SSH på Raven, eller i skyen.

## Oppstart av hver ny økt

1. Kjør `git pull` i repo-roten før du leser eller endrer noe.
2. Les det nyeste handover-dokumentet i `docs/` (høyest nummer — per nå
   `docs/HANDOVER-1-TIL-2.md`) og `README.md`. Der står status, beslutninger
   og hva som er neste milepæl.
3. All kommunikasjon med brukeren skjer på norsk. Forklar tekniske valg
   enkelt og konkret.

## Arkitekturprinsipper (skal ikke brytes)

- **Session-controlleren** (backend) er den eneste komponenten som starter
  eller stopper workspace-containere, og kun gjennom de faste operasjonene
  create/start/stop/status/destroy. Frontend skal aldri kunne sende
  vilkårlige Docker-parametre.
- Workspace-containere får **aldri**: docker-socket, `--privileged`,
  host-filsystem, host-nettverk eller GPU (RTX 5080).
- Tjenesten er kun for godkjent lokalnett/Tailscale. Ikke eksponer porter
  mot internett, og ikke legg til login/kontoer uten eksplisitt bestilling.
- `/wall` er alltid read-only: joiner aldri, sender aldri input.

## Raven-spesifikt (maskinen tjenesten kjører på)

- GUI kjører på Intel UHD 770 (hovedkortets utganger). **RTX 5080 er
  reservert CUDA/AI** — aldri koble skjerm til den, og aldri reverser
  `nvidia-drm modeset=0`-innstillingen.
- Ikke flytt skjermkabler i drift: kabelflytting gir null-skjerm-øyeblikk
  som krasjer GNOME (se `docs/KABLET-OPPKOBLING.md`).
- Wall-kiosk-skriptene i `wall/` må kjøres i desktop-sessionen på Raven,
  aldri over SSH (de trenger DISPLAY).

## Design

- Følg KODE15s designprofil: farger, fonter og variabler er definert i
  `frontend/src/app.css` (lys lobby, mørk «stage», Bebas Neue + Open Sans).
  Ikke innfør nye farger eller fonter uten grunn.

## Arbeidsflyt

- Små, fokuserte PR-er mot `main` med beskrivende norske titler.
- Ved drifts- eller oppsettendringer: oppdater `docs/` i samme PR.
- Uløste ting og lærdommer noteres i handover-notatene, ikke bare i chatten.
- Test det du kan uten Docker med `WORKSPACE_DRIVER=mock` (se README,
  «Utvikling uten Docker»).
