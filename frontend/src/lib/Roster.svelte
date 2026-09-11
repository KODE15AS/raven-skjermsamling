<script>
  // Live deltagerliste med sanntidsstatus: farge, rolle, workspace-tilstand,
  // minimize og hvem som kontrollerer hvem.
  let { participants = [], youId = null } = $props();

  function byId(id) {
    return participants.find((p) => p.id === id);
  }

  function statusText(p) {
    if (!p.connected) return 'frakoblet …';
    if (p.role === 'observer') return 'observer';
    const s = p.workspace?.state;
    if (s === 'starting') return 'starter Chrome …';
    if (s === 'error') return p.workspace.message || 'feil';
    if (s === 'running') {
      if (p.minimized) return 'minimert';
      if (p.controls) {
        const t = byId(p.controls);
        return `kontrollerer ${t ? t.name : '…'}`;
      }
      return 'aktiv';
    }
    return 'venter';
  }
</script>

<ul class="roster">
  {#each participants as p (p.id)}
    <li class:me={p.id === youId} class:offline={!p.connected}>
      <span class="dot" style="background: {p.color}"></span>
      <span class="name">{p.name}{p.id === youId ? ' (deg)' : ''}</span>
      <span class="state" class:starting={p.workspace?.state === 'starting'}>
        {statusText(p)}
      </span>
      {#if p.controlled_by}
        {@const c = byId(p.controlled_by)}
        {#if c}
          <span class="ctrl" style="border-color: {c.color}; color: {c.color}">
            {c.name} kontrollerer
          </span>
        {/if}
      {/if}
    </li>
  {:else}
    <li class="empty">Ingen deltagere ennå</li>
  {/each}
</ul>

<style>
  .roster {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.55rem 0.8rem;
    font-size: 0.95rem;
  }

  li.me {
    border-color: color-mix(in srgb, var(--text-dim) 50%, var(--border));
  }

  li.offline {
    opacity: 0.5;
  }

  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex: none;
  }

  .name {
    font-weight: 600;
  }

  .state {
    margin-left: auto;
    color: var(--text-dim);
    font-size: 0.8rem;
  }

  .state.starting {
    animation: pulse 1.5s infinite;
  }

  .ctrl {
    font-size: 0.72rem;
    border: 1px solid;
    border-radius: 999px;
    padding: 0.1rem 0.5rem;
  }

  .empty {
    color: var(--text-dim);
    justify-content: center;
  }
</style>
