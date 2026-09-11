import { writable, derived, get } from 'svelte/store';

// --- Tilstand ---------------------------------------------------------------

export const connected = writable(false);
export const you = writable(null); // egen Participant fra welcome
export const participants = writable([]);
export const maxActive = writable(4);
export const activeWorkspaces = writable(0);
export const lastError = writable(null);

// id -> { tile, x, y, color, name, ts } – koordinater normalisert innenfor
// tilen til deltageren `tile`, så pekere treffer riktig i alle layouts.
export const cursors = writable(new Map());

export const deltagere = derived(participants, (p) => p.filter((x) => x.role === 'deltager'));

let ws = null;
let reconnectTimer = null;
let watchMode = false;
let joinPayload = null;

const SESSION_KEY = 'skjermsamling.session';
const NAME_KEY = 'skjermsamling.name';
const ROLE_KEY = 'skjermsamling.role';

export const savedName = () => localStorage.getItem(NAME_KEY) || '';
export const savedRole = () => localStorage.getItem(ROLE_KEY) || 'deltager';

function wsUrl(watch) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  return `${proto}://${location.host}/ws${watch ? '?watch=1' : ''}`;
}

function handleMessage(ev) {
  let msg;
  try {
    msg = JSON.parse(ev.data);
  } catch {
    return;
  }
  switch (msg.type) {
    case 'welcome':
      you.set(msg.you);
      localStorage.setItem(SESSION_KEY, msg.session);
      break;
    case 'roster': {
      participants.set(msg.participants);
      maxActive.set(msg.max_active);
      activeWorkspaces.set(msg.active_workspaces);
      const me = get(you);
      if (me) {
        const updated = msg.participants.find((p) => p.id === me.id);
        if (updated) you.set(updated);
      }
      break;
    }
    case 'cursor': {
      const me = get(you);
      if (me && msg.id === me.id) break; // ikke vis egen ghost-cursor
      cursors.update((m) => {
        const next = new Map(m);
        next.set(msg.id, {
          tile: msg.tile,
          x: msg.x,
          y: msg.y,
          color: msg.color,
          name: msg.name,
          ts: Date.now(),
        });
        return next;
      });
      break;
    }
    case 'error':
      lastError.set(msg.message);
      setTimeout(() => lastError.set(null), 5000);
      break;
  }
}

function openSocket() {
  ws = new WebSocket(wsUrl(watchMode));
  ws.onopen = () => {
    connected.set(true);
    if (!watchMode && joinPayload) {
      const session = localStorage.getItem(SESSION_KEY) || undefined;
      ws.send(JSON.stringify({ type: 'join', ...joinPayload, session }));
    }
  };
  ws.onmessage = handleMessage;
  ws.onclose = () => {
    connected.set(false);
    // Automatisk reconnect: sessionen overlever på serveren til timeout.
    reconnectTimer = setTimeout(openSocket, 1500);
  };
  ws.onerror = () => ws && ws.close();
}

// --- Handlinger -------------------------------------------------------------

export function join(name, role) {
  localStorage.setItem(NAME_KEY, name);
  localStorage.setItem(ROLE_KEY, role);
  joinPayload = { name, role };
  if (watchMode) {
    // Oppgrader fra read-only forhåndsvisning til ekte deltagelse.
    watchMode = false;
    if (ws) {
      ws.onclose = null;
      ws.close();
      ws = null;
    }
    openSocket();
    return;
  }
  if (ws && ws.readyState === WebSocket.OPEN) {
    const session = localStorage.getItem(SESSION_KEY) || undefined;
    ws.send(JSON.stringify({ type: 'join', name, role, session }));
  } else if (!ws || ws.readyState > WebSocket.OPEN) {
    openSocket();
  }
}

/** Koble til som read-only seer (70"-veggen). Joiner aldri. */
export function watch() {
  watchMode = true;
  joinPayload = null;
  if (!ws || ws.readyState > WebSocket.OPEN) openSocket();
}

/** Reconnect med lagret navn/rolle hvis vi har en tidligere session. */
export function rejoinIfPossible() {
  const name = savedName();
  if (name && localStorage.getItem(SESSION_KEY)) {
    join(name, savedRole());
    return true;
  }
  return false;
}

function send(obj) {
  if (ws && ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify(obj));
}

let lastCursorSent = 0;
/** Send egen pekerposisjon, normalisert innenfor tilen til deltager `tile`. */
export function sendCursor(tile, x, y) {
  const now = performance.now();
  if (now - lastCursorSent < 33) return; // ~30 Hz
  lastCursorSent = now;
  send({ type: 'cursor', tile, x, y });
}

export const minimize = () => send({ type: 'minimize' });
export const restore = () => send({ type: 'restore' });
export const takeControl = (target) => send({ type: 'control', target });
export const releaseControl = () => send({ type: 'release' });

export function leave() {
  send({ type: 'leave' });
  localStorage.removeItem(SESSION_KEY);
  you.set(null);
  if (reconnectTimer) clearTimeout(reconnectTimer);
  if (ws) {
    ws.onclose = null;
    ws.close();
    ws = null;
  }
  connected.set(false);
}

/** Bytt ut {host}-plassholder i workspace-URL med nettleserens vertsnavn. */
export function resolveUrl(url) {
  return url ? url.replace('{host}', location.hostname) : url;
}

// Rydd bort ghost-cursors som ikke har beveget seg på en stund.
setInterval(() => {
  cursors.update((m) => {
    const now = Date.now();
    let changed = false;
    const next = new Map(m);
    for (const [id, c] of next) {
      if (now - c.ts > 8000) {
        next.delete(id);
        changed = true;
      }
    }
    return changed ? next : m;
  });
}, 2000);
