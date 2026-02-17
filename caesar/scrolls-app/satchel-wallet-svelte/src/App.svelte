<script>
  import { Router } from '@roxi/routify'
  import { routes } from '../.routify/routes.default.js'
  import Navigation from './lib/components/Navigation.svelte'
</script>

<div class="app">
  <Navigation />
  <main class="main-content">
    <Router {routes} />
  </main>
</div>

<style>
  .app {
    min-height: 100vh;
    background: var(--dark-bg);
    display: flex;
    flex-direction: column;
  }

  .main-content {
    flex: 1;
    padding: 20px;
    max-width: 1200px;
    margin: 0 auto;
    width: 100%;
  }
</style>