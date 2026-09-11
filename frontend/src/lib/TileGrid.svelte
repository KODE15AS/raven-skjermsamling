<script>
  // Responsivt tile-grid: layout tilpasser seg automatisk antall aktive
  // tiles (1 / 2 / 2x2 / 3x2 / 3x3). Deles av /samling («Alle») og /wall.
  // Ghost-pekere spores og rendres per tile (se Tile/Cursors).
  import Tile from './Tile.svelte';

  let {
    participants = [],
    viewer = null,
    readonly = false,
    onTakeControl = () => {},
    onRelease = () => {},
    onMinimize = () => {},
    onRestore = () => {},
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
</script>

<div class="grid-wrap">
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
          {onRestore}
        />
      {/each}
    </div>
  {/if}
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
