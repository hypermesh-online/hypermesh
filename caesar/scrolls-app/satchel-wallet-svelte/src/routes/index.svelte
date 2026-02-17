<script>
  import UniversalAssetDashboard from '../lib/components/assets/UniversalAssetDashboard.svelte'
  import { onMount } from 'svelte'
  import { assetStore } from '../lib/stores/assets.js'
  
  onMount(() => {
    assetStore.initializeAssetManager()
  })
</script>

<svelte:head>
  <title>HyperMesh Assets - Universal Real-World Economy Manager</title>
  <meta name="description" content="Manage your real-world assets, execute contracts, and participate in the Caesar economy through HyperMesh" />
</svelte:head>

<UniversalAssetDashboard />