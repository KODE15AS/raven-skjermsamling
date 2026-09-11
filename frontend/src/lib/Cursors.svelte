<script>
  // Fargede ghost-cursors: HTML-overlay (ikke OS-pekere) posisjonert med
  // normaliserte koordinater relativt til tile-området.
  import { cursors } from './store.js';
</script>

<div class="cursors" aria-hidden="true">
  {#each [...$cursors] as [id, c] (id)}
    <div class="cursor" style="left: {c.x * 100}%; top: {c.y * 100}%">
      <svg width="22" height="22" viewBox="0 0 24 24">
        <path
          d="M4 2 L20 12 L12 13.5 L8.5 21 Z"
          fill={c.color}
          stroke="#0c1116"
          stroke-width="1.5"
        />
      </svg>
      <span class="label" style="background: {c.color}">{c.name}</span>
    </div>
  {/each}
</div>

<style>
  .cursors {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: 50;
  }

  .cursor {
    position: absolute;
    transform: translate(-2px, -2px);
    transition: left 0.05s linear, top 0.05s linear;
    display: flex;
    align-items: flex-start;
  }

  .label {
    margin-top: 14px;
    margin-left: -4px;
    color: #0c1116;
    font-size: 0.72rem;
    font-weight: 700;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    white-space: nowrap;
  }
</style>
