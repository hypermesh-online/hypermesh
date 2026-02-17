<script>
  import { onMount } from 'svelte';
  import { 
    asset_registry,
    caesar_economy,
    assetStore,
    asset_manager_connected,
    ASSET_TYPES
  } from '../../lib/stores/assets.js';
  import { 
    ShoppingCart,
    TrendingUp,
    Home,
    Car,
    Cpu,
    HardDrive,
    Search,
    Filter,
    Star,
    MapPin,
    Clock,
    DollarSign,
    Shield
  } from 'lucide-svelte';
  import Card from '../../lib/components/ui/Card.svelte';
  import Button from '../../lib/components/ui/Button.svelte';
  import Input from '../../lib/components/ui/Input.svelte';

  let searchQuery = '';
  let selectedCategory = 'all';
  let priceRange = 'all';
  let location = 'all';

  // Mock marketplace data for real asset trading
  const mockMarketplaceListings = [
    {
      id: 'listing-001',
      type: ASSET_TYPES.VEHICLE,
      title: '2020 Honda Civic - Shared Access',
      description: 'Well-maintained Honda Civic available for time-sharing. Perfect for city commuting.',
      price: 45,
      currency: 'CAESAR',
      period: 'per day',
      location: 'San Francisco, CA',
      availability: 'Available weekdays',
      rating: 4.8,
      reviews: 24,
      owner: 'verified_user_123',
      features: ['GPS Navigation', 'Bluetooth', 'Backup Camera', 'Good MPG'],
      images: ['civic-1.jpg'],
      sharing_type: 'time_share',
      verification_level: 'full_consensus'
    },
    {
      id: 'listing-002',
      type: ASSET_TYPES.REAL_ESTATE,
      title: 'Co-working Space - Hourly Rent',
      description: 'Professional co-working space with high-speed internet and meeting rooms.',
      price: 12,
      currency: 'CAESAR',
      period: 'per hour',
      location: 'Austin, TX',
      availability: 'Mon-Fri 9AM-6PM',
      rating: 4.9,
      reviews: 67,
      owner: 'workspace_provider_456',
      features: ['High-speed WiFi', 'Meeting rooms', 'Printing', 'Coffee'],
      images: ['workspace-1.jpg'],
      sharing_type: 'hourly_access',
      verification_level: 'property_verified'
    },
    {
      id: 'listing-003',
      type: ASSET_TYPES.STORAGE,
      title: 'Distributed Storage Pool Access',
      description: 'Quantum-encrypted distributed storage with global CDN access.',
      price: 2.5,
      currency: 'CAESAR',
      period: 'per TB/month',
      location: 'Global Network',
      availability: '24/7',
      rating: 5.0,
      reviews: 156,
      owner: 'hypermesh_storage_789',
      features: ['Quantum encryption', 'Global CDN', '99.9% uptime', 'Auto-sharding'],
      images: ['storage-1.jpg'],
      sharing_type: 'storage_access',
      verification_level: 'hypermesh_validated'
    },
    {
      id: 'listing-004',
      type: ASSET_TYPES.EQUIPMENT,
      title: 'Professional Camera Kit',
      description: 'Complete professional photography kit for events and projects.',
      price: 85,
      currency: 'CAESAR',
      period: 'per day',
      location: 'Los Angeles, CA',
      availability: 'Available weekends',
      rating: 4.7,
      reviews: 43,
      owner: 'photographer_pro_321',
      features: ['4K Video', 'Multiple lenses', 'Tripods', 'Lighting kit'],
      images: ['camera-1.jpg'],
      sharing_type: 'equipment_rental',
      verification_level: 'owner_verified'
    }
  ];

  // Filter listings
  $: filteredListings = mockMarketplaceListings.filter(listing => {
    const matchesSearch = listing.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         listing.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = selectedCategory === 'all' || listing.type === selectedCategory;
    
    return matchesSearch && matchesCategory;
  });

  function getAssetIcon(assetType) {
    switch(assetType) {
      case ASSET_TYPES.REAL_ESTATE: return Home;
      case ASSET_TYPES.VEHICLE: return Car;
      case ASSET_TYPES.CPU: return Cpu;
      case ASSET_TYPES.STORAGE: return HardDrive;
      case ASSET_TYPES.EQUIPMENT: return ShoppingCart;
      default: return ShoppingCart;
    }
  }

  function getVerificationBadge(level) {
    switch(level) {
      case 'full_consensus':
        return { text: 'Full Consensus', class: 'bg-green-500/20 text-green-400' };
      case 'hypermesh_validated':
        return { text: 'HyperMesh', class: 'bg-blue-500/20 text-blue-400' };
      case 'property_verified':
        return { text: 'Property Verified', class: 'bg-yellow-500/20 text-yellow-400' };
      case 'owner_verified':
        return { text: 'Owner Verified', class: 'bg-purple-500/20 text-purple-400' };
      default:
        return { text: 'Basic', class: 'bg-gray-500/20 text-gray-400' };
    }
  }

  onMount(() => {
    if ($asset_manager_connected) {
      // Load user's assets and marketplace data
    }
  });
</script>

<svelte:head>
  <title>Asset Trading - HyperMesh Marketplace</title>
  <meta name="description" content="Trade real-world assets, share resources, and exchange value in the CAESAR economy" />
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold caesar-gradient">Asset Trading Marketplace</h1>
      <p class="text-gray-400 mt-2">
        Trade real assets, share resources, and participate in the CAESAR economy
      </p>
    </div>
    
    <Button>
      <ShoppingCart class="w-4 h-4 mr-2" />
      List Asset
    </Button>
  </div>

  {#if !$asset_manager_connected}
    <!-- Connection Prompt -->
    <Card class="text-center py-12">
      <div class="space-y-4">
        <ShoppingCart class="w-16 h-16 text-yellow-400 mx-auto" />
        <h2 class="text-xl font-semibold">Connect to HyperMesh</h2>
        <p class="text-gray-400 max-w-md mx-auto">
          Connect to the HyperMesh ecosystem to access the asset trading marketplace.
        </p>
        <Button on:click={() => assetStore.connectAssetManager()} class="mt-4">
          Connect HyperMesh
        </Button>
      </div>
    </Card>
  {:else}
    <!-- Trading Overview -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-6">
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">CAESAR Balance</p>
            <p class="text-2xl font-bold text-white">{$caesar_economy.balance}</p>
          </div>
          <DollarSign class="w-8 h-8 text-yellow-400" />
        </div>
      </Card>

      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Available Listings</p>
            <p class="text-2xl font-bold text-white">{mockMarketplaceListings.length}</p>
          </div>
          <ShoppingCart class="w-8 h-8 text-blue-400" />
        </div>
      </Card>

      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Your Assets</p>
            <p class="text-2xl font-bold text-white">{$asset_registry ? $asset_registry.size : 0}</p>
          </div>
          <Home class="w-8 h-8 text-green-400" />
        </div>
      </Card>

      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Anti-Speculation</p>
            <p class="text-2xl font-bold text-white">{$caesar_economy.anti_speculation_score}%</p>
          </div>
          <Shield class="w-8 h-8 text-purple-400" />
        </div>
      </Card>
    </div>

    <!-- Search and Filters -->
    <Card class="p-6">
      <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
        <!-- Search -->
        <div class="relative">
          <Search class="w-4 h-4 absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" />
          <Input
            bind:value={searchQuery}
            placeholder="Search assets..."
            class="pl-10"
          />
        </div>

        <!-- Category Filter -->
        <select 
          bind:value={selectedCategory}
          class="px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent"
        >
          <option value="all">All Categories</option>
          <option value={ASSET_TYPES.VEHICLE}>Vehicles</option>
          <option value={ASSET_TYPES.REAL_ESTATE}>Real Estate</option>
          <option value={ASSET_TYPES.EQUIPMENT}>Equipment</option>
          <option value={ASSET_TYPES.STORAGE}>Storage</option>
          <option value={ASSET_TYPES.CPU}>Computing</option>
        </select>

        <!-- Price Range -->
        <select 
          bind:value={priceRange}
          class="px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent"
        >
          <option value="all">All Prices</option>
          <option value="low">Under 25 CAESAR</option>
          <option value="medium">25-100 CAESAR</option>
          <option value="high">Over 100 CAESAR</option>
        </select>

        <!-- Location -->
        <select 
          bind:value={location}
          class="px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent"
        >
          <option value="all">All Locations</option>
          <option value="local">Local Area</option>
          <option value="remote">Remote Access</option>
          <option value="global">Global Network</option>
        </select>
      </div>
    </Card>

    <!-- Marketplace Listings -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      {#each filteredListings as listing (listing.id)}
        <Card class="p-6 hover:border-yellow-400/50 transition-all duration-200 hover:shadow-lg hover:shadow-yellow-400/10">
          <div class="flex items-start space-x-4">
            <!-- Asset Icon -->
            <div class="w-16 h-16 bg-gray-800 rounded-lg flex items-center justify-center flex-shrink-0">
              <svelte:component this={getAssetIcon(listing.type)} class="w-8 h-8 text-yellow-400" />
            </div>

            <!-- Listing Details -->
            <div class="flex-1">
              <div class="flex items-start justify-between mb-2">
                <div>
                  <h3 class="text-lg font-semibold text-white">{listing.title}</h3>
                  <p class="text-sm text-gray-400 capitalize">{listing.type.replace('_', ' ')}</p>
                </div>
                
                <!-- Verification Badge -->
                <span class="px-2 py-1 text-xs rounded-full {getVerificationBadge(listing.verification_level).class}">
                  {getVerificationBadge(listing.verification_level).text}
                </span>
              </div>

              <!-- Description -->
              <p class="text-gray-300 text-sm mb-3 line-clamp-2">{listing.description}</p>

              <!-- Price and Details -->
              <div class="grid grid-cols-2 gap-4 mb-3">
                <div>
                  <p class="text-2xl font-bold text-white">
                    {listing.price} CAESAR
                  </p>
                  <p class="text-xs text-gray-400">{listing.period}</p>
                </div>
                
                <div class="text-right">
                  <div class="flex items-center justify-end space-x-1 mb-1">
                    <Star class="w-4 h-4 text-yellow-400" />
                    <span class="text-sm font-medium text-white">{listing.rating}</span>
                    <span class="text-xs text-gray-400">({listing.reviews})</span>
                  </div>
                </div>
              </div>

              <!-- Location and Availability -->
              <div class="flex items-center space-x-4 mb-3 text-sm text-gray-400">
                <div class="flex items-center space-x-1">
                  <MapPin class="w-4 h-4" />
                  <span>{listing.location}</span>
                </div>
                <div class="flex items-center space-x-1">
                  <Clock class="w-4 h-4" />
                  <span>{listing.availability}</span>
                </div>
              </div>

              <!-- Features -->
              <div class="mb-4">
                <div class="flex flex-wrap gap-2">
                  {#each listing.features.slice(0, 3) as feature}
                    <span class="px-2 py-1 text-xs bg-gray-800 text-gray-300 rounded-full">
                      {feature}
                    </span>
                  {/each}
                  {#if listing.features.length > 3}
                    <span class="px-2 py-1 text-xs bg-gray-700 text-gray-400 rounded-full">
                      +{listing.features.length - 3} more
                    </span>
                  {/if}
                </div>
              </div>

              <!-- Action Buttons -->
              <div class="flex space-x-2">
                <Button size="sm" variant="outline" class="flex-1">
                  View Details
                </Button>
                <Button size="sm" class="flex-1">
                  Request Access
                </Button>
              </div>
            </div>
          </div>
        </Card>
      {/each}
    </div>

    {#if filteredListings.length === 0}
      <Card class="text-center py-12">
        <Search class="w-16 h-16 text-gray-400 mx-auto mb-4" />
        <h2 class="text-xl font-semibold text-gray-300">No Listings Found</h2>
        <p class="text-gray-400 mt-2">
          {searchQuery ? `No assets match "${searchQuery}"` : 'No assets match your current filters'}
        </p>
        <Button on:click={() => { searchQuery = ''; selectedCategory = 'all'; }} class="mt-4">
          Clear Filters
        </Button>
      </Card>
    {/if}

    <!-- Real Economy Trading Benefits -->
    <Card class="p-6 bg-gradient-to-r from-green-500/10 to-green-600/10 border-green-400/20">
      <div class="flex items-start space-x-4">
        <div class="w-12 h-12 bg-green-400/20 rounded-lg flex items-center justify-center">
          <TrendingUp class="w-6 h-6 text-green-400" />
        </div>
        <div>
          <h3 class="font-semibold text-white mb-2">Real-World Asset Economy</h3>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div>
              <p class="text-green-400 font-medium">True Utility Trading</p>
              <p class="text-gray-300">
                Trade access to real assets like vehicles, property, and equipment - not speculative tokens.
              </p>
            </div>
            <div>
              <p class="text-green-400 font-medium">Consensus Validation</p>
              <p class="text-gray-300">
                All trades validated with HyperMesh consensus (PoSpace + PoStake + PoWork + PoTime).
              </p>
            </div>
            <div>
              <p class="text-green-400 font-medium">Anti-Speculation Design</p>
              <p class="text-gray-300">
                CAESAR demurrage encourages productive use over speculation, keeping prices fair.
              </p>
            </div>
          </div>
          
          <div class="mt-4 p-3 bg-gray-800/50 rounded-lg">
            <p class="text-xs text-gray-400">
              Your trading reputation score: <span class="text-green-400 font-medium">{$caesar_economy.anti_speculation_score}%</span>
              • Higher scores get priority access and better rates.
            </p>
          </div>
        </div>
      </div>
    </Card>
  {/if}
</div>

<style>
  .caesar-gradient {
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>