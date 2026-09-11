<script>
  import Lobby from './views/Lobby.svelte';
  import Samling from './views/Samling.svelte';
  import Wall from './views/Wall.svelte';

  // Enkel path-basert ruting: /, /samling, /wall.
  let path = $state(location.pathname);

  function navigate(to) {
    history.pushState({}, '', to);
    path = to;
  }

  window.addEventListener('popstate', () => {
    path = location.pathname;
  });
</script>

{#if path.startsWith('/wall')}
  <Wall />
{:else if path.startsWith('/samling')}
  <Samling {navigate} />
{:else}
  <Lobby {navigate} />
{/if}
