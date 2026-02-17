<script>
  import { onMount } from 'svelte';
  import { 
    privacy_config,
    assetStore,
    asset_manager_connected,
    PRIVACY_LEVELS
  } from '../../lib/stores/assets.js';
  import { 
    Settings,
    Shield,
    User,
    Bell,
    Globe,
    Lock,
    Eye,
    Users,
    Zap,
    Save,
    RotateCcw
  } from 'lucide-svelte';
  import Card from '../../lib/components/ui/Card.svelte';
  import Button from '../../lib/components/ui/Button.svelte';
  import Input from '../../lib/components/ui/Input.svelte';

  let activeTab = 'privacy';
  let hasUnsavedChanges = false;
  
  // Local settings state for form handling
  let localPrivacyConfig = { ...$privacy_config };
  let notificationSettings = {
    consensus_alerts: true,
    payment_reminders: true,
    sharing_requests: true,
    system_updates: false,
    marketing: false
  };

  function getPrivacyLevelIcon(level) {
    switch(level) {
      case PRIVACY_LEVELS.PRIVATE: return Lock;
      case PRIVACY_LEVELS.PRIVATE_NETWORK: return Shield;
      case PRIVACY_LEVELS.P2P: return Users;
      case PRIVACY_LEVELS.PUBLIC_NETWORK: return Eye;
      case PRIVACY_LEVELS.FULL_PUBLIC: return Globe;
      default: return Shield;
    }
  }

  function getPrivacyLevelColor(level) {
    switch(level) {
      case PRIVACY_LEVELS.PRIVATE: return 'text-red-400';
      case PRIVACY_LEVELS.PRIVATE_NETWORK: return 'text-orange-400';
      case PRIVACY_LEVELS.P2P: return 'text-yellow-400';
      case PRIVACY_LEVELS.PUBLIC_NETWORK: return 'text-blue-400';
      case PRIVACY_LEVELS.FULL_PUBLIC: return 'text-green-400';
      default: return 'text-gray-400';
    }
  }

  function getPrivacyDescription(level) {
    switch(level) {
      case PRIVACY_LEVELS.PRIVATE: 
        return 'Internal network only, no external access';
      case PRIVACY_LEVELS.PRIVATE_NETWORK: 
        return 'Specific networks/groups only';
      case PRIVACY_LEVELS.P2P: 
        return 'Trusted peer sharing';
      case PRIVACY_LEVELS.PUBLIC_NETWORK: 
        return 'Specific public networks';
      case PRIVACY_LEVELS.FULL_PUBLIC: 
        return 'Maximum CAESAR rewards, full HyperMesh node';
      default: 
        return 'Privacy level not set';
    }
  }

  function handlePrivacyChange() {
    hasUnsavedChanges = true;
  }

  async function saveSettings() {
    try {
      // Update privacy config in store
      privacy_config.set(localPrivacyConfig);
      
      // TODO: Send settings to HyperMesh network
      hasUnsavedChanges = false;
      
      // Show success notification
      console.log('Settings saved successfully');
    } catch (error) {
      console.error('Failed to save settings:', error);
    }
  }

  function resetSettings() {
    localPrivacyConfig = { ...$privacy_config };
    hasUnsavedChanges = false;
  }

  // Watch for changes in privacy config
  $: if (localPrivacyConfig !== $privacy_config) {
    handlePrivacyChange();
  }

  onMount(() => {
    localPrivacyConfig = { ...$privacy_config };
  });
</script>

<svelte:head>
  <title>Settings - HyperMesh Asset Manager</title>
  <meta name="description" content="Configure your HyperMesh privacy, security, and notification settings" />
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold caesar-gradient">Settings</h1>
      <p class="text-gray-400 mt-2">
        Configure your HyperMesh privacy, security, and preferences
      </p>
    </div>
    
    {#if hasUnsavedChanges}
      <div class="flex space-x-2">
        <Button variant="outline" on:click={resetSettings}>
          <RotateCcw class="w-4 h-4 mr-2" />
          Reset
        </Button>
        <Button on:click={saveSettings}>
          <Save class="w-4 h-4 mr-2" />
          Save Changes
        </Button>
      </div>
    {/if}
  </div>

  {#if !$asset_manager_connected}
    <!-- Connection Prompt -->
    <Card class="text-center py-12">
      <div class="space-y-4">
        <Settings class="w-16 h-16 text-yellow-400 mx-auto" />
        <h2 class="text-xl font-semibold">Connect to HyperMesh</h2>
        <p class="text-gray-400 max-w-md mx-auto">
          Connect to the HyperMesh ecosystem to access your settings and preferences.
        </p>
        <Button on:click={() => assetStore.connectAssetManager()} class="mt-4">
          Connect HyperMesh
        </Button>
      </div>
    </Card>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
      <!-- Settings Navigation -->
      <Card class="p-6 h-fit">
        <h2 class="font-semibold text-white mb-4">Settings</h2>
        <nav class="space-y-2">
          <button
            on:click={() => activeTab = 'privacy'}
            class="w-full flex items-center space-x-3 px-3 py-2 rounded-md text-left transition-colors {activeTab === 'privacy' ? 'bg-yellow-400/10 text-yellow-400' : 'text-gray-300 hover:text-white hover:bg-gray-800'}"
          >
            <Shield class="w-4 h-4" />
            <span>Privacy & Security</span>
          </button>
          
          <button
            on:click={() => activeTab = 'profile'}
            class="w-full flex items-center space-x-3 px-3 py-2 rounded-md text-left transition-colors {activeTab === 'profile' ? 'bg-yellow-400/10 text-yellow-400' : 'text-gray-300 hover:text-white hover:bg-gray-800'}"
          >
            <User class="w-4 h-4" />
            <span>Profile</span>
          </button>
          
          <button
            on:click={() => activeTab = 'notifications'}
            class="w-full flex items-center space-x-3 px-3 py-2 rounded-md text-left transition-colors {activeTab === 'notifications' ? 'bg-yellow-400/10 text-yellow-400' : 'text-gray-300 hover:text-white hover:bg-gray-800'}"
          >
            <Bell class="w-4 h-4" />
            <span>Notifications</span>
          </button>
          
          <button
            on:click={() => activeTab = 'advanced'}
            class="w-full flex items-center space-x-3 px-3 py-2 rounded-md text-left transition-colors {activeTab === 'advanced' ? 'bg-yellow-400/10 text-yellow-400' : 'text-gray-300 hover:text-white hover:bg-gray-800'}"
          >
            <Zap class="w-4 h-4" />
            <span>Advanced</span>
          </button>
        </nav>
      </Card>

      <!-- Settings Content -->
      <div class="lg:col-span-3 space-y-6">
        {#if activeTab === 'privacy'}
          <!-- Privacy & Security Settings -->
          <Card class="p-6">
            <div class="flex items-center space-x-3 mb-6">
              <Shield class="w-6 h-6 text-yellow-400" />
              <h2 class="text-xl font-semibold">Privacy & Security</h2>
            </div>

            <!-- Default Privacy Level -->
            <div class="mb-8">
              <label class="block text-lg font-medium text-white mb-4">
                Default Privacy Level
              </label>
              <p class="text-sm text-gray-400 mb-6">
                Choose how your assets and resources are shared by default. You can override this for individual assets.
              </p>
              
              <div class="space-y-3">
                {#each Object.values(PRIVACY_LEVELS) as level}
                  <div class="flex items-center space-x-4 p-4 border border-gray-700 rounded-lg hover:border-gray-600 transition-colors">
                    <input
                      type="radio"
                      name="default_privacy"
                      value={level}
                      bind:group={localPrivacyConfig.default_level}
                      on:change={handlePrivacyChange}
                      class="text-yellow-400 focus:ring-yellow-400"
                    />
                    <div class="flex items-center space-x-3 flex-1">
                      <div class="w-8 h-8 bg-gray-800 rounded-lg flex items-center justify-center">
                        <svelte:component this={getPrivacyLevelIcon(level)} class="w-4 h-4 {getPrivacyLevelColor(level)}" />
                      </div>
                      <div>
                        <p class="font-medium text-white capitalize">{level.replace('_', ' ')}</p>
                        <p class="text-sm text-gray-400">{getPrivacyDescription(level)}</p>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>

            <!-- Sharing Limits -->
            <div class="mb-8">
              <h3 class="text-lg font-medium text-white mb-4">Resource Sharing Limits</h3>
              
              <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">
                    Maximum Concurrent Users
                  </label>
                  <Input
                    type="number"
                    min="1"
                    max="100"
                    bind:value={localPrivacyConfig.max_concurrent_users}
                    on:input={handlePrivacyChange}
                    class="w-full"
                  />
                  <p class="text-xs text-gray-400 mt-1">
                    How many users can access your shared resources simultaneously
                  </p>
                </div>

                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-2">
                    Consent Duration (hours)
                  </label>
                  <Input
                    type="number"
                    min="1"
                    max="168"
                    bind:value={localPrivacyConfig.consent_duration}
                    on:input={handlePrivacyChange}
                    class="w-full"
                  />
                  <p class="text-xs text-gray-400 mt-1">
                    How long sharing permissions last before requiring renewal
                  </p>
                </div>
              </div>
            </div>

            <!-- Security Options -->
            <div>
              <h3 class="text-lg font-medium text-white mb-4">Security Options</h3>
              
              <div class="space-y-4">
                <div class="flex items-center space-x-3">
                  <input
                    type="checkbox"
                    bind:checked={localPrivacyConfig.require_consensus}
                    on:change={handlePrivacyChange}
                    class="text-yellow-400 focus:ring-yellow-400"
                  />
                  <div>
                    <label class="font-medium text-white">
                      Require Full Consensus Validation
                    </label>
                    <p class="text-sm text-gray-400">
                      All asset operations must validate all four consensus proofs (PoSpace + PoStake + PoWork + PoTime)
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </Card>

        {:else if activeTab === 'profile'}
          <!-- Profile Settings -->
          <Card class="p-6">
            <div class="flex items-center space-x-3 mb-6">
              <User class="w-6 h-6 text-yellow-400" />
              <h2 class="text-xl font-semibold">Profile Settings</h2>
            </div>

            <div class="space-y-6">
              <div>
                <label class="block text-sm font-medium text-gray-300 mb-2">
                  Display Name
                </label>
                <Input
                  type="text"
                  placeholder="Your display name"
                  class="w-full max-w-md"
                />
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-300 mb-2">
                  Bio
                </label>
                <textarea
                  placeholder="Tell others about yourself..."
                  rows="4"
                  class="w-full max-w-md px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent resize-none"
                ></textarea>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-300 mb-2">
                  Location
                </label>
                <Input
                  type="text"
                  placeholder="City, Country"
                  class="w-full max-w-md"
                />
              </div>
            </div>
          </Card>

        {:else if activeTab === 'notifications'}
          <!-- Notification Settings -->
          <Card class="p-6">
            <div class="flex items-center space-x-3 mb-6">
              <Bell class="w-6 h-6 text-yellow-400" />
              <h2 class="text-xl font-semibold">Notification Preferences</h2>
            </div>

            <div class="space-y-6">
              <div class="flex items-center justify-between">
                <div>
                  <p class="font-medium text-white">Consensus Alerts</p>
                  <p class="text-sm text-gray-400">Get notified when consensus validation fails or degrades</p>
                </div>
                <input
                  type="checkbox"
                  bind:checked={notificationSettings.consensus_alerts}
                  class="text-yellow-400 focus:ring-yellow-400"
                />
              </div>

              <div class="flex items-center justify-between">
                <div>
                  <p class="font-medium text-white">Payment Reminders</p>
                  <p class="text-sm text-gray-400">Reminders for upcoming contract payments and renewals</p>
                </div>
                <input
                  type="checkbox"
                  bind:checked={notificationSettings.payment_reminders}
                  class="text-yellow-400 focus:ring-yellow-400"
                />
              </div>

              <div class="flex items-center justify-between">
                <div>
                  <p class="font-medium text-white">Sharing Requests</p>
                  <p class="text-sm text-gray-400">Notifications when others request access to your assets</p>
                </div>
                <input
                  type="checkbox"
                  bind:checked={notificationSettings.sharing_requests}
                  class="text-yellow-400 focus:ring-yellow-400"
                />
              </div>

              <div class="flex items-center justify-between">
                <div>
                  <p class="font-medium text-white">System Updates</p>
                  <p class="text-sm text-gray-400">Updates about HyperMesh network and protocol changes</p>
                </div>
                <input
                  type="checkbox"
                  bind:checked={notificationSettings.system_updates}
                  class="text-yellow-400 focus:ring-yellow-400"
                />
              </div>

              <div class="flex items-center justify-between">
                <div>
                  <p class="font-medium text-white">Marketing & Tips</p>
                  <p class="text-sm text-gray-400">Tips for optimizing your asset sharing and CAESAR earnings</p>
                </div>
                <input
                  type="checkbox"
                  bind:checked={notificationSettings.marketing}
                  class="text-yellow-400 focus:ring-yellow-400"
                />
              </div>
            </div>
          </Card>

        {:else if activeTab === 'advanced'}
          <!-- Advanced Settings -->
          <Card class="p-6">
            <div class="flex items-center space-x-3 mb-6">
              <Zap class="w-6 h-6 text-yellow-400" />
              <h2 class="text-xl font-semibold">Advanced Configuration</h2>
            </div>

            <!-- Quantum Security Status -->
            <div class="mb-8">
              <h3 class="text-lg font-medium text-white mb-4">Quantum-Resistant Security</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg">
                  <div>
                    <p class="font-medium text-white">FALCON-1024 Signatures</p>
                    <p class="text-sm text-gray-400">Post-quantum digital signatures</p>
                  </div>
                  <span class="text-green-400 font-medium">Enabled</span>
                </div>
                
                <div class="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg">
                  <div>
                    <p class="font-medium text-white">Kyber Encryption</p>
                    <p class="text-sm text-gray-400">Post-quantum key encapsulation</p>
                  </div>
                  <span class="text-green-400 font-medium">Enabled</span>
                </div>
              </div>
            </div>

            <!-- Network Configuration -->
            <div class="mb-8">
              <h3 class="text-lg font-medium text-white mb-4">Network Configuration</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg">
                  <div>
                    <p class="font-medium text-white">Remote Proxy NAT</p>
                    <p class="text-sm text-gray-400">Memory addressing system</p>
                  </div>
                  <span class="text-blue-400 font-medium">Active</span>
                </div>
                
                <div class="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg">
                  <div>
                    <p class="font-medium text-white">STOQ Protocol</p>
                    <p class="text-sm text-gray-400">High-performance transport</p>
                  </div>
                  <span class="text-green-400 font-medium">Connected</span>
                </div>
              </div>
            </div>

            <!-- Ecosystem Integration -->
            <div>
              <h3 class="text-lg font-medium text-white mb-4">Ecosystem Integration</h3>
              <div class="space-y-3">
                <div class="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg">
                  <div>
                    <p class="font-medium text-white">TrustChain Certificates</p>
                    <p class="text-sm text-gray-400">Certificate authority validation</p>
                  </div>
                  <span class="text-green-400 font-medium">Validated</span>
                </div>
                
                <div class="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg">
                  <div>
                    <p class="font-medium text-white">Catalog JuliaVM</p>
                    <p class="text-sm text-gray-400">Secure contract execution engine</p>
                  </div>
                  <span class="text-green-400 font-medium">Ready</span>
                </div>
                
                <div class="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg">
                  <div>
                    <p class="font-medium text-white">HyperMesh Core</p>
                    <p class="text-sm text-gray-400">Main orchestration platform</p>
                  </div>
                  <span class="text-green-400 font-medium">Connected</span>
                </div>
              </div>
            </div>
          </Card>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .caesar-gradient {
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
</style>