<script>
  // Responsivt tile-grid: layout tilpasser seg automatisk antall aktive
  // tiles (1 / 2 / 2x2 / 3x2 / 3x3). Deles av /samling og /wall.
  import Tile from './Tile.svelte';
  import Cursors from './Cursors.svelte';
  import { sendCursor } from './store.js';

  let {
    participants = [],
    viewer = null,
    readonly = false,
    onTakeControl = () => {},
    onRelease = () => {},
    onMinimize = () => {},
  } = $props();

  const withWorkspace = $derived(
    participants.filter(
      (p) => p.role === 'deltager' && p.workspace && p.workspace.state !== 'none'
    )
  );
  const visible = $derived(withWorkspace.filter((p) => !p.minimized));

  const cols = $derived(
    visible.length <= 1 ? 1 : visible.length <= 4 ? 2 : 3
  );

  function controllerOf(p) {
    if (!p.controlled_by) return null;
    return participants.find((x) => x.id === p.controlled_by) || null;
  }

  let gridEl = $state(null);

  function onMove(e) {
    if (readonly || !gridEl) return;
    const r = gridEl.getBoundingClientRect();
    const x = (e.clientX - r.left) / r.width;
    const y = (e.clientY - r.top) / r.height;
    if (x >= 0 && x <= 1 && y >= 0 && y <= 1) sendCursor(x, y);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="grid-wrap"
  bind:this={gridEl}
  onmousemove={onMove}
>
  {#if visible.length === 0}
    <div class="empty">
      <h2>Ingen aktive workspaces</h2>
      <p>Tiles dukker opp her når deltagere blir med.</p>
    </div>
  {:else}
    <div class="grid" style="--cols: {cols}">
      {#each visible as p (p.id)}
        <Tile
          {p}
          {viewer}
          {readonly}
          controller={controllerOf(p)}
          {onTakeControl}
          {onRelease}
          {onMinimize}
        />
      {/each}
    </div>
  {/if}
  <Cursors />
</div>

<style>
  .grid-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    padding: 10px;
  }

  .grid {
    height: 100%;
    display: grid;
    grid-template-columns: repeat(var(--cols), 1fr);
    grid-auto-rows: 1fr;
    gap: 12px;
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-dim);
  }

  .empty h2 {
    margin: 0 0 0.4rem;
    color: var(--text);
  }
</style>
