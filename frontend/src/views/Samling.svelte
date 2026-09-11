<script>
  // /samling har to visninger:
  //  - «Min skjerm» (standard for deltagere): kun egen/fokusert workspace i
  //    fullt format. De andre er navnechips i topplinjen — og på 70"-veggen.
  //    Klikk på en chip for å se/peke på/ta over den personens workspace.
  //  - «Alle»: grid med alle aktive tiles (som på veggen), med minimize-bar.
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
  import Tile from '../lib/Tile.svelte';

  let { navigate } = $props();

  // Åpnet i egen tab: gjenopprett session fra localStorage.
  $effect(() => {
    if (!rejoinIfPossible()) navigate('/');
  });

  const MODE_KEY = 'skjermsamling.visning';
  let mode = $state(localStorage.getItem(MODE_KEY) || 'min');
  let focusedId = $state(null); // null = egen workspace

  function setMode(m) {
    mode = m;
    localStorage.setItem(MODE_KEY, m);
    if (m === 'alle') focusedId = null;
  }

  function focusParticipant(p) {
    if (!$you || p.id === $you.id) {
      focusedId = null;
      setMode('min');
      return;
    }
    if (p.role !== 'deltager' || p.workspace?.state !== 'running') return;
    focusedId = p.id;
    setMode('min');
  }

  function backToMine() {
    if ($you?.controls) releaseControl();
    focusedId = null;
  }

  // Deltageren som vises i «Min skjerm»-modus, med fallback til egen.
  const focusedP = $derived.by(() => {
    if (focusedId) {
      const p = $participants.find((x) => x.id === focusedId);
      if (p && p.workspace?.state !== 'none') return p;
    }
    return $you ? $participants.find((x) => x.id === $you.id) : null;
  });

  const minimized = $derived(
    $participants.filter(
      (p) => p.role === 'deltager' && p.minimized && p.workspace?.state !== 'none'
    )
  );

  const isObserver = $derived($you?.role === 'observer');

  // Observere har ingen egen workspace; «Alle» er naturlig standard for dem.
  $effect(() => {
    if (isObserver && !focusedId && mode === 'min') setMode('alle');
  });

  function onKey(e) {
    if (e.key !== 'Escape') return;
    if ($you?.controls) {
      releaseControl();
    } else if (mode === 'min' && focusedId) {
      backToMine();
    }
  }

  function chipClickable(p) {
    return (
      p.role === 'deltager' &&
      p.workspace?.state === 'running' &&
      (!$you || p.id !== $you.id)
    );
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="samling dark">
  <header class="topbar">
    <button class="home" onclick={() => navigate('/')} title="Til lobby">
      Skjermsamling
    </button>

    <div class="chips">
      {#each $participants as p (p.id)}
        <button
          class="chip"
          class:offline={!p.connected}
          class:focused={mode === 'min' && focusedP && p.id === focusedP.id}
          class:clickable={chipClickable(p)}
          style="--c: {p.color}"
          title={chipClickable(p)
            ? `Se og pek på ${p.name} sin workspace`
            : `${p.name} (${p.role})`}
          onclick={() => focusParticipant(p)}
        >
          {p.name}
        </button>
      {/each}
    </div>

    <div class="modes" role="radiogroup" aria-label="Visning">
      <button
        class:active={mode === 'min'}
        onclick={() => setMode('min')}
        title="Kun din egen workspace – samlingen ser du på 70-tommeren"
      >
        Min skjerm
      </button>
      <button
        class:active={mode === 'alle'}
        onclick={() => setMode('alle')}
        title="Alle aktive workspaces som grid"
      >
        Alle
      </button>
    </div>

    <span class="conn" class:ok={$connected}></span>
  </header>

  {#if $lastError}
    <div class="toast">{$lastError}</div>
  {/if}

  {#if $you?.controls}
    <div class="control-banner">
      Du kontrollerer en annen workspace – klikk «Avslutt kontroll» på tilen,
      eller trykk <kbd>Esc</kbd>
    </div>
  {:else if mode === 'min' && focusedId && focusedP && $you && focusedP.id !== $you.id}
    <div class="focus-banner" style="--c: {focusedP.color}">
      Du ser {focusedP.name} sin workspace – pek med musen, dobbeltklikk for å
      kontrollere
      <button class="back" onclick={backToMine}>Tilbake til min skjerm</button>
    </div>
  {/if}

  {#if mode === 'min'}
    <div class="solo">
      {#if focusedP}
        {#key focusedP.id}
          <Tile
            p={focusedP}
            viewer={$you}
            controller={focusedP.controlled_by
              ? $participants.find((x) => x.id === focusedP.controlled_by) || null
              : null}
            onTakeControl={takeControl}
            onRelease={releaseControl}
            onMinimize={minimize}
            onRestore={restore}
          />
        {/key}
      {:else}
        <div class="none">
          <h2>{isObserver ? 'Velg en deltager i topplinjen' : 'Ingen workspace ennå'}</h2>
          <p>
            {isObserver
              ? 'Som observer kan du se andres workspaces – eller bytt til «Alle».'
              : 'Workspacen din dukker opp her når den er klar.'}
          </p>
        </div>
      {/if}
    </div>
  {:else}
    <TileGrid
      participants={$participants}
      viewer={$you}
      onTakeControl={takeControl}
      onRelease={releaseControl}
      onMinimize={minimize}
      onRestore={restore}
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
    background: rgba(38, 50, 70, 0.97); /* KODE15-navy */
    border-bottom: 1px solid var(--border);
  }

  .home {
    background: none;
    color: #f3f1ec;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: 1.15rem;
    letter-spacing: 0.05em;
    padding: 0;
  }

  .home:hover {
    color: var(--k15-beige);
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
    background: transparent;
    color: #f3f1ec;
    cursor: default;
  }

  .chip::before {
    content: '';
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--c);
  }

  .chip.clickable {
    cursor: pointer;
  }

  .chip.clickable:hover {
    background: color-mix(in srgb, var(--c) 25%, transparent);
  }

  .chip.focused {
    background: color-mix(in srgb, var(--c) 35%, transparent);
    font-weight: 700;
  }

  .chip.offline {
    opacity: 0.45;
  }

  .modes {
    display: flex;
    border: 1px solid rgba(243, 241, 236, 0.35);
    border-radius: 999px;
    overflow: hidden;
  }

  .modes button {
    background: transparent;
    color: #f3f1ec;
    font-size: 0.78rem;
    padding: 0.22rem 0.8rem;
  }

  .modes button.active {
    background: var(--k15-beige);
    color: var(--k15-navy);
    font-weight: 700;
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
    background: var(--success);
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
    background: color-mix(in srgb, var(--k15-beige) 22%, var(--bg-elev));
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
    padding: 0.3rem;
  }

  .focus-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.8rem;
    background: color-mix(in srgb, var(--c) 16%, var(--bg-elev));
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
    padding: 0.25rem;
  }

  .focus-banner .back {
    background: var(--c);
    color: #0c1116;
    font-weight: 700;
    font-size: 0.78rem;
    padding: 0.15rem 0.7rem;
    border-radius: 999px;
  }

  kbd {
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 0.35rem;
    font-family: inherit;
  }

  .solo {
    flex: 1;
    min-height: 0;
    display: flex;
    padding: 10px;
  }

  .solo :global(.tile) {
    flex: 1;
  }

  .none {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-dim);
  }

  .none h2 {
    margin: 0 0 0.4rem;
    color: var(--text);
  }

  .minbar {
    display: flex;
    gap: 0.5rem;
    padding: 0.45rem 0.9rem;
    background: rgba(38, 50, 70, 0.97);
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
