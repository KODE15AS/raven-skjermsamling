<script>
  // Én workspace-tile: Selkies/KasmVNC-stream i iframe med permanent
  // eierfarge-ramme, ev. ytre kontrollramme og eksplisitt kontroll-UX.
  import { resolveUrl } from './store.js';

  let {
    p, // deltageren som eier workspacen
    viewer = null, // deg selv (Participant) eller null (wall)
    controller = null, // deltageren som ev. kontrollerer denne workspacen
    readonly = false, // /wall: aldri input
    onTakeControl = () => {},
    onRelease = () => {},
    onMinimize = () => {},
  } = $props();

  const isOwner = $derived(viewer && viewer.id === p.id);
  const iControl = $derived(viewer && viewer.controls === p.id);
  // Input til streamen tillates kun for eier og for aktiv controller.
  const interactive = $derived(!readonly && (isOwner || iControl));
  const canTake = $derived(
    !readonly &&
      viewer &&
      viewer.role === 'deltager' &&
      !isOwner &&
      !iControl &&
      !controller &&
      p.workspace?.state === 'running'
  );

  const url = $derived(
    p.workspace?.state === 'running' ? resolveUrl(p.workspace.url) : null
  );

  function overlayDblClick() {
    if (canTake) onTakeControl(p.id);
    else if (iControl) onRelease();
  }
</script>

<div
  class="tile"
  class:minimized-hidden={p.minimized}
  style="--owner: {p.color}; --controller: {controller ? controller.color : 'transparent'}"
>
  {#if controller}
    <div class="control-frame" aria-hidden="true"></div>
    <div class="control-badge" style="background: {controller.color}">
      {controller.name} kontrollerer
    </div>
  {/if}

  <header class="bar">
    <span class="owner-dot" style="background: {p.color}"></span>
    <span class="owner-name">{p.name}</span>
    {#if !p.connected}
      <span class="offline">frakoblet</span>
    {/if}
    <span class="spacer"></span>
    {#if iControl}
      <button class="mini act" onclick={onRelease} title="Avslutt kontroll (Esc)">
        Avslutt kontroll
      </button>
    {/if}
    {#if isOwner && !readonly}
      <button class="mini" onclick={onMinimize} title="Minimer din tile">–</button>
    {/if}
  </header>

  <div class="viewport">
    {#if url}
      <iframe
        src={url}
        title="Workspace: {p.name}"
        style="pointer-events: {interactive ? 'auto' : 'none'}"
        allow="clipboard-read; clipboard-write"
      ></iframe>
      {#if !interactive && !readonly}
        <!-- Fanger dobbeltklikk for eksplisitt kontroll-overtagelse -->
        <div
          class="overlay"
          role="button"
          tabindex="-1"
          ondblclick={overlayDblClick}
        >
          {#if canTake}
            <span class="hint">Dobbeltklikk for å kontrollere</span>
          {:else if controller && viewer && controller.id !== viewer.id}
            <span class="hint dim">{controller.name} kontrollerer</span>
          {/if}
        </div>
      {/if}
      {#if iControl}
        <!-- Klikk-transparent markør slik at man kan avslutte med dobbeltklikk på ramme -->
        <div class="controlling-edge" ondblclick={overlayDblClick} role="button" tabindex="-1"></div>
      {/if}
    {:else if p.workspace?.state === 'starting'}
      <div class="placeholder">
        <div class="spinner" style="border-top-color: {p.color}"></div>
        <span>Starter Chrome på Raven …</span>
      </div>
    {:else if p.workspace?.state === 'error'}
      <div class="placeholder err">
        <span>⚠ {p.workspace.message}</span>
      </div>
    {:else}
      <div class="placeholder"><span>Venter …</span></div>
    {/if}
  </div>
</div>

<style>
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    border: 3px solid var(--owner); /* permanent eierramme */
    border-radius: 10px;
    overflow: hidden;
    background: #000;
    min-height: 0;
  }

  .control-frame {
    position: absolute;
    inset: -3px;
    border: 3px solid var(--controller); /* ytre kontrollramme */
    border-radius: 12px;
    outline: 3px solid var(--controller);
    outline-offset: 3px;
    pointer-events: none;
    z-index: 5;
  }

  .control-badge {
    position: absolute;
    top: -1px;
    left: 50%;
    transform: translateX(-50%);
    color: #0c1116;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 0.15rem 0.7rem;
    border-radius: 0 0 8px 8px;
    z-index: 6;
    white-space: nowrap;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    background: color-mix(in srgb, var(--owner) 14%, #10161c);
    font-size: 0.85rem;
    z-index: 2;
  }

  .owner-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
  }

  .owner-name {
    font-weight: 700;
  }

  .offline {
    color: var(--text-dim);
    font-size: 0.75rem;
    animation: pulse 1.5s infinite;
  }

  .spacer {
    flex: 1;
  }

  .mini {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text);
    border-radius: 6px;
    padding: 0.1rem 0.55rem;
    font-size: 0.8rem;
    line-height: 1.4;
  }

  .mini:hover {
    background: rgba(255, 255, 255, 0.18);
  }

  .mini.act {
    background: var(--controller);
    color: #0c1116;
    font-weight: 700;
  }

  .viewport {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  iframe {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: 0;
    background: #101418;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    z-index: 3;
  }

  .overlay .hint {
    opacity: 0;
    background: rgba(12, 17, 22, 0.85);
    border: 1px solid var(--border);
    padding: 0.5rem 1rem;
    border-radius: 999px;
    font-size: 0.9rem;
    transition: opacity 0.15s;
    pointer-events: none;
  }

  .overlay:hover .hint {
    opacity: 1;
  }

  .overlay .hint.dim {
    font-size: 0.8rem;
  }

  .controlling-edge {
    position: absolute;
    inset: 0 0 auto 0;
    height: 14px;
    z-index: 4;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.8rem;
    color: var(--text-dim);
  }

  .placeholder.err {
    color: var(--danger);
    text-align: center;
    padding: 1rem;
  }

  .spinner {
    width: 34px;
    height: 34px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .minimized-hidden {
    display: none;
  }
</style>
