<script>
  import {
    join,
    leave,
    connected,
    you,
    participants,
    maxActive,
    activeWorkspaces,
    lastError,
    savedName,
    savedRole,
    rejoinIfPossible,
    watch,
  } from '../lib/store.js';
  import Roster from '../lib/Roster.svelte';

  let { navigate } = $props();

  let name = $state(savedName());
  let role = $state(savedRole());

  // Kom tilbake til eksisterende session automatisk, ellers koble til
  // read-only for å vise live-status før man blir med.
  if (!rejoinIfPossible()) watch();

  function submit(e) {
    e.preventDefault();
    if (!name.trim()) return;
    join(name.trim(), role);
  }

  function openSamling() {
    // Best-effort gjenbruk av eksisterende tab via navngitt vindu.
    const w = window.open('/samling', 'skjermsamling');
    if (w) w.focus();
  }

  function leaveSession() {
    leave();
  }
</script>

<main class="lobby">
  <section class="card panel">
    <header class="brand">
      <div class="mark" aria-hidden="true">
        <span></span><span></span><span></span><span></span>
      </div>
      <div>
        <h1>SKJERMSAMLING</h1>
        <p class="sub">KODE15 · Raven · felles interaktiv arbeidsflate</p>
      </div>
    </header>

    {#if $lastError}
      <div class="error">{$lastError}</div>
    {/if}

    {#if !$you}
      <form onsubmit={submit}>
        <label for="name">Navn / kallenavn</label>
        <input
          id="name"
          type="text"
          bind:value={name}
          maxlength="32"
          placeholder="F.eks. Øystein"
          autocomplete="off"
        />

        <div class="roles" role="radiogroup" aria-label="Rolle">
          <button
            type="button"
            class:active={role === 'deltager'}
            onclick={() => (role = 'deltager')}
          >
            <strong>Deltager</strong>
            <small>Egen Chrome på Raven + kontroll</small>
          </button>
          <button
            type="button"
            class:active={role === 'observer'}
            onclick={() => (role = 'observer')}
          >
            <strong>Observer</strong>
            <small>Ser alt, sender ikke input</small>
          </button>
        </div>

        <button class="btn-primary wide" type="submit" disabled={!name.trim()}>
          Bli med
        </button>

        {#if $participants.length > 0}
          <div class="preview">
            <span class="preview-label">Inne nå</span>
            <Roster participants={$participants} youId={null} />
          </div>
        {/if}
      </form>
    {:else}
      <div class="joined">
        <div class="me">
          <span class="dot" style="background: {$you.color}"></span>
          <span class="me-name">{$you.name}</span>
          <span class="me-role">{$you.role === 'deltager' ? 'Deltager' : 'Observer'}</span>
        </div>

        <button class="btn-primary wide" onclick={openSamling}>
          Gå til Skjermsamling
        </button>

        <div class="capacity">
          {$activeWorkspaces} av {$maxActive} workspaces aktive
        </div>

        <Roster participants={$participants} youId={$you.id} />

        <button class="btn-ghost wide" onclick={leaveSession}>Forlat Skjermsamling</button>
      </div>
    {/if}

    <footer class="status">
      <span class="conn" class:ok={$connected}></span>
      {$connected ? 'Tilkoblet Raven' : 'Kobler til …'}
    </footer>
  </section>
</main>

<style>
  .lobby {
    min-height: 100%;
    display: grid;
    place-items: center;
    padding: 2rem 1rem;
  }

  .panel {
    width: min(30rem, 100%);
    padding: 2.2rem;
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .mark {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    width: 44px;
    height: 44px;
  }

  .mark span {
    border-radius: 5px;
  }

  .mark span:nth-child(1) { background: #ffd400; }
  .mark span:nth-child(2) { background: #2ecc40; }
  .mark span:nth-child(3) { background: #339cff; }
  .mark span:nth-child(4) { background: #ff4136; }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 2.1rem;
    font-weight: 400;
    letter-spacing: 0.04em;
    line-height: 1;
    color: var(--k15-muted);
  }

  .sub {
    margin: 0.1rem 0 0;
    color: var(--text-dim);
    font-size: 0.9rem;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  label {
    font-size: 0.85rem;
    color: var(--text-dim);
  }

  input {
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 1.1rem;
    padding: 0.8rem 1rem;
    outline: none;
    transition: border-color 0.15s;
  }

  input:focus {
    border-color: var(--k15-beige);
  }

  .roles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.7rem;
  }

  .roles button {
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    padding: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    text-align: left;
    transition: border-color 0.15s, background 0.15s;
  }

  .roles button small {
    color: var(--text-dim);
  }

  .roles button.active {
    border-color: var(--k15-accent);
    background: color-mix(in srgb, var(--k15-beige) 22%, var(--bg-elev-2));
  }

  .wide {
    width: 100%;
  }

  .joined {
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }

  .me {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 1.1rem;
  }

  .dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    flex: none;
  }

  .me-name {
    font-weight: 700;
  }

  .me-role {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin-left: auto;
  }

  .capacity {
    color: var(--text-dim);
    font-size: 0.85rem;
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    border-top: 1px solid var(--border);
    padding-top: 0.9rem;
  }

  .preview-label {
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
  }

  .error {
    background: color-mix(in srgb, var(--danger) 12%, var(--bg-elev));
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    padding: 0.6rem 0.9rem;
    font-size: 0.9rem;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-dim);
    font-size: 0.8rem;
    border-top: 1px solid var(--border);
    padding-top: 1rem;
  }

  .conn {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--danger);
    animation: pulse 1.5s infinite;
  }

  .conn.ok {
    background: var(--success);
    animation: none;
  }
</style>
