<script>
  // Multi-screen-visningen: alle aktive workspaces som tiles, minimize-bar
  // nederst, demokratisk kontroll via dobbeltklikk og Esc for å slippe.
  import {
    participants,
    you,
    connected,
    lastError,
    minimize,
    restore,
    takeControl,
    releaseControl,
    rejoinIfPossible,
  } from '../lib/store.js';
  import TileGrid from '../lib/TileGrid.svelte';

  let { navigate } = $props();

  // Åpnet i egen tab: gjenopprett session fra localStorage.
  $effect(() => {
    if (!rejoinIfPossible()) navigate('/');
  });

  const minimized = $derived(
    $participants.filter(
      (p) => p.role === 'deltager' && p.minimized && p.workspace?.state !== 'none'
    )
  );

  function onKey(e) {
    if (e.key === 'Escape' && $you?.controls) releaseControl();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="samling">
  <header class="topbar">
    <button class="home" onclick={() => navigate('/')} title="Til lobby">
      Skjermsamling
    </button>
    <div class="chips">
      {#each $participants as p (p.id)}
        <span
          class="chip"
          class:offline={!p.connected}
          style="--c: {p.color}"
          title="{p.name} ({p.role})"
        >
          {p.name}
        </span>
      {/each}
    </div>
    <span class="conn" class:ok={$connected}></span>
  </header>

  {#if $lastError}
    <div class="toast">{$lastError}</div>
  {/if}

  {#if $you?.controls}
    <div class="control-banner">
      Du kontrollerer en annen workspace – trykk <kbd>Esc</kbd> for å avslutte
    </div>
  {/if}

  <TileGrid
    participants={$participants}
    viewer={$you}
    onTakeControl={takeControl}
    onRelease={releaseControl}
    onMinimize={minimize}
  />

  {#if minimized.length > 0}
    <footer class="minbar">
      {#each minimized as p (p.id)}
        <button
          class="mintab"
          style="--c: {p.color}"
          disabled={p.id !== $you?.id}
          onclick={() => p.id === $you?.id && restore()}
          title={p.id === $you?.id ? 'Gjenopprett din tile' : `${p.name} er minimert`}
        >
          {p.name}
        </button>
      {/each}
    </footer>
  {/if}
</div>

<style>
  .samling {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.45rem 0.9rem;
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
  }

  .home {
    background: none;
    color: var(--text);
    font-weight: 800;
    font-size: 0.95rem;
    letter-spacing: -0.01em;
    padding: 0;
  }

  .home:hover {
    color: var(--accent);
  }

  .chips {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    flex: 1;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.78rem;
    padding: 0.12rem 0.6rem;
    border-radius: 999px;
    border: 1.5px solid var(--c);
    color: var(--text);
  }

  .chip::before {
    content: '';
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--c);
  }

  .chip.offline {
    opacity: 0.45;
  }

  .conn {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--danger);
    animation: pulse 1.5s infinite;
    flex: none;
  }

  .conn.ok {
    background: #2ecc40;
    animation: none;
  }

  .toast {
    position: fixed;
    top: 3.2rem;
    left: 50%;
    transform: translateX(-50%);
    background: color-mix(in srgb, var(--danger) 20%, var(--bg-elev));
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    padding: 0.5rem 1.1rem;
    z-index: 100;
    font-size: 0.9rem;
  }

  .control-banner {
    text-align: center;
    background: color-mix(in srgb, var(--accent) 14%, var(--bg-elev));
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
    padding: 0.3rem;
  }

  kbd {
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 0.35rem;
    font-family: inherit;
  }

  .minbar {
    display: flex;
    gap: 0.5rem;
    padding: 0.45rem 0.9rem;
    background: var(--bg-elev);
    border-top: 1px solid var(--border);
  }

  .mintab {
    background: color-mix(in srgb, var(--c) 18%, var(--bg-elev-2));
    border: 2px solid var(--c);
    color: var(--text);
    font-weight: 700;
    font-size: 0.85rem;
    padding: 0.3rem 1rem;
    border-radius: 8px;
  }

  .mintab:disabled {
    cursor: default;
    opacity: 0.75;
  }

  .mintab:not(:disabled):hover {
    background: color-mix(in srgb, var(--c) 35%, var(--bg-elev-2));
  }
</style>
