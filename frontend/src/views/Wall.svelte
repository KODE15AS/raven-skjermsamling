<script>
  // /wall – Raven Wall for 70"-skjermen: kjøres i Chromium kiosk på Ravens
  // egen Ubuntu-desktop (motherboard-HDMI / UHD 770). Alltid read-only,
  // sender aldri tastatur/mus-input.
  import { participants, connected, watch } from '../lib/store.js';
  import TileGrid from '../lib/TileGrid.svelte';

  watch();

  const clock = $state({ t: fmt() });

  function fmt() {
    return new Date().toLocaleTimeString('nb-NO', {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  setInterval(() => (clock.t = fmt()), 10_000);
</script>

<div class="wall">
  <TileGrid participants={$participants} viewer={null} readonly={true} />

  <footer class="wallbar">
    <span class="brand">Skjermsamling · Raven</span>
    <div class="people">
      {#each $participants as p (p.id)}
        <span class="pill" class:offline={!p.connected} style="--c: {p.color}">
          {p.name}
        </span>
      {/each}
    </div>
    <span class="clock">{clock.t}</span>
    <span class="conn" class:ok={$connected} title={$connected ? 'Tilkoblet' : 'Kobler til'}></span>
  </footer>
</div>

<style>
  .wall {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #05080b;
    cursor: none; /* veggen har ingen egen peker */
  }

  .wallbar {
    display: flex;
    align-items: center;
    gap: 1.2rem;
    padding: 0.5rem 1.2rem;
    background: #0a0f14;
    border-top: 1px solid #182029;
    font-size: 1rem;
  }

  .brand {
    font-weight: 800;
    letter-spacing: -0.01em;
    color: var(--text-dim);
  }

  .people {
    flex: 1;
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    border: 2px solid var(--c);
    border-radius: 999px;
    padding: 0.15rem 0.8rem;
    font-weight: 700;
  }

  .pill::before {
    content: '';
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--c);
  }

  .pill.offline {
    opacity: 0.4;
  }

  .clock {
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }

  .conn {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--danger);
    animation: pulse 1.5s infinite;
  }

  .conn.ok {
    background: #2ecc40;
    animation: none;
  }
</style>
