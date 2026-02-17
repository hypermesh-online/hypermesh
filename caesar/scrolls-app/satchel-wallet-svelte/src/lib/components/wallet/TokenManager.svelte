<script>
  import { 
    wallet_connected,
    token_balances,
    caesar_token_data
  } from '../../stores/wallet.js';
  import Card from '../ui/Card.svelte';
  import Button from '../ui/Button.svelte';
  import Input from '../ui/Input.svelte';
  import { 
    Settings,
    ArrowLeft,
    Plus,
    Trash2,
    AlertTriangle,
    Info,
    TrendingDown
  } from 'lucide-svelte';
  import { url } from '@roxi/routify';
  
  let showAddToken = false;
  let newTokenAddress = '';
  let newTokenSymbol = '';
  let addingToken = false;
  
  $: tokenList = Array.from($token_balances.entries()).map(([symbol, balance]) => ({
    symbol,
    balance: parseFloat(balance),
    isDefault: ['ETH', 'CAESAR'].includes(symbol)
  }));
  
  function formatBalance(balance) {
    return balance.toFixed(6).replace(/\.?0+$/, '');
  }
  
  function calculatePortfolioValue() {
    // TODO: Implement actual USD value calculation
    return tokenList.reduce((total, token) => total + token.balance * 100, 0); // Mock USD values
  }
  
  async function addCustomToken() {
    if (!newTokenAddress || !newTokenSymbol) return;
    
    try {
      addingToken = true;
      
      // TODO: Implement actual token validation and adding
      await new Promise(resolve => setTimeout(resolve, 1500));
      
      // Mock adding token
      token_balances.update(balances => {
        const newBalances = new Map(balances);
        newBalances.set(newTokenSymbol, '0.00');
        return newBalances;
      });
      
      // Reset form
      newTokenAddress = '';
      newTokenSymbol = '';
      showAddToken = false;
      
    } catch (error) {
      console.error('Failed to add token:', error);
    } finally {
      addingToken = false;
    }
  }
  
  function removeToken(symbol) {
    token_balances.update(balances => {
      const newBalances = new Map(balances);
      newBalances.delete(symbol);
      return newBalances;
    });
  }
  
  function getTokenIcon(symbol) {
    // Return appropriate icon or first letter
    return symbol.charAt(0);
  }
  
  function getTokenDescription(symbol) {
    switch (symbol) {
      case 'CAESAR':
        return 'Anti-speculation currency with demurrage mechanics';
      case 'ETH':
        return 'Native Ethereum token for gas and DeFi';
      default:
        return 'Custom ERC-20 token';
    }
  }
</script>

<div class="max-w-4xl mx-auto space-y-6">
  <!-- Header -->
  <div class="flex items-center space-x-4">
    <a href={$url('/')}>
      <Button variant="ghost" size="icon">
        <ArrowLeft class="w-4 h-4" />
      </Button>
    </a>
    <div class="flex-1">
      <h1 class="text-2xl font-bold text-white">Token Management</h1>
      <p class="text-gray-400">Manage your token portfolio and settings</p>
    </div>
    <Button variant="outline" size="sm" on:click={() => showAddToken = true}>
      <Plus class="w-4 h-4 mr-2" />
      Add Token
    </Button>
  </div>

  {#if !$wallet_connected}
    <Card class="text-center py-8">
      <p class="text-gray-400">Please connect your wallet to manage tokens</p>
    </Card>
  {:else}
    <!-- Portfolio Overview -->
    <Card>
      <h3 class="text-lg font-semibold text-white mb-4">Portfolio Overview</h3>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="text-center">
          <p class="text-sm text-gray-400">Total Tokens</p>
          <p class="text-2xl font-bold text-white">{tokenList.length}</p>
        </div>
        <div class="text-center">
          <p class="text-sm text-gray-400">Estimated Value</p>
          <p class="text-2xl font-bold caesar-gradient">${calculatePortfolioValue().toFixed(2)}</p>
        </div>
        <div class="text-center">
          <p class="text-sm text-gray-400">Active Networks</p>
          <p class="text-2xl font-bold text-white">1</p>
        </div>
      </div>
    </Card>

    <!-- Caesar Token Special Section -->
    <Card class="border-yellow-500/30 bg-yellow-500/5">
      <div class="flex items-start space-x-3">
        <div class="w-12 h-12 bg-gradient-to-r from-yellow-400 to-yellow-600 rounded-full flex items-center justify-center">
          <span class="text-black font-bold">C</span>
        </div>
        <div class="flex-1">
          <h3 class="text-lg font-semibold text-yellow-400 mb-2">Caesar Token Configuration</h3>
          <div class="space-y-3 text-sm">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <p class="text-gray-400">Current Balance</p>
                <p class="text-white font-medium">{$caesar_token_data.effective_balance} CAESAR</p>
              </div>
              <div>
                <p class="text-gray-400">Demurrage Rate</p>
                <p class="text-white font-medium">{($caesar_token_data.demurrage_rate * 100).toFixed(1)}% annually</p>
              </div>
              <div>
                <p class="text-gray-400">Last Activity</p>
                <p class="text-white font-medium">{new Date($caesar_token_data.last_activity).toLocaleDateString()}</p>
              </div>
              <div>
                <p class="text-gray-400">Accumulated Demurrage</p>
                <p class="text-red-400 font-medium">
                  {$caesar_token_data.accumulated_demurrage > 0 ? '-' : ''}{$caesar_token_data.accumulated_demurrage.toFixed(4)} CAESAR
                </p>
              </div>
            </div>
            
            {#if $caesar_token_data.accumulated_demurrage > 1.0}
              <div class="bg-amber-500/10 border border-amber-500/30 rounded-lg p-3 mt-4">
                <div class="flex items-start space-x-2">
                  <TrendingDown class="w-4 h-4 text-amber-400 mt-0.5" />
                  <div>
                    <p class="text-amber-400 font-medium">High Demurrage Alert</p>
                    <p class="text-gray-300 text-xs">
                      Consider using your Caesar tokens to minimize demurrage effects. 
                      Regular economic activity helps maintain token value.
                    </p>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>
    </Card>

    <!-- Token List -->
    <Card>
      <h3 class="text-lg font-semibold text-white mb-4">Your Tokens</h3>
      <div class="space-y-3">
        {#each tokenList as token}
          <div class="flex items-center justify-between p-4 bg-gray-800 rounded-lg">
            <div class="flex items-center space-x-4">
              <div class="w-10 h-10 bg-gradient-to-r from-yellow-400 to-yellow-600 rounded-full flex items-center justify-center">
                <span class="text-black font-bold text-sm">{getTokenIcon(token.symbol)}</span>
              </div>
              <div>
                <div class="flex items-center space-x-2">
                  <h4 class="font-semibold text-white">{token.symbol}</h4>
                  {#if token.isDefault}
                    <span class="text-xs px-2 py-1 bg-gray-700 text-gray-300 rounded-full">Default</span>
                  {/if}
                </div>
                <p class="text-sm text-gray-400">{getTokenDescription(token.symbol)}</p>
              </div>
            </div>
            
            <div class="flex items-center space-x-4">
              <div class="text-right">
                <p class="font-semibold text-white">{formatBalance(token.balance)}</p>
                <p class="text-sm text-gray-400">{token.symbol}</p>
              </div>
              
              {#if !token.isDefault}
                <Button 
                  variant="ghost" 
                  size="icon"
                  on:click={() => removeToken(token.symbol)}
                  title="Remove token"
                >
                  <Trash2 class="w-4 h-4 text-red-400" />
                </Button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </Card>

    <!-- Add Custom Token -->
    {#if showAddToken}
      <Card>
        <h3 class="text-lg font-semibold text-white mb-4">Add Custom Token</h3>
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Token Contract Address</label>
            <Input
              bind:value={newTokenAddress}
              placeholder="0x..."
              class="w-full"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Token Symbol</label>
            <Input
              bind:value={newTokenSymbol}
              placeholder="e.g. USDC"
              class="w-full"
            />
          </div>
          
          <div class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-3">
            <div class="flex items-start space-x-2">
              <Info class="w-4 h-4 text-blue-400 mt-0.5" />
              <p class="text-sm text-gray-300">
                Make sure the token contract address is correct. Adding malicious tokens can be dangerous.
              </p>
            </div>
          </div>
          
          <div class="flex space-x-3">
            <Button 
              variant="default"
              on:click={addCustomToken}
              disabled={!newTokenAddress || !newTokenSymbol || addingToken}
              class="flex-1"
            >
              {#if addingToken}
                Adding Token...
              {:else}
                <Plus class="w-4 h-4 mr-2" />
                Add Token
              {/if}
            </Button>
            <Button 
              variant="outline"
              on:click={() => showAddToken = false}
              disabled={addingToken}
            >
              Cancel
            </Button>
          </div>
        </div>
      </Card>
    {/if}

    <!-- Network Information -->
    <Card>
      <h3 class="text-lg font-semibold text-white mb-4">Network Information</h3>
      <div class="space-y-3 text-sm">
        <div class="flex justify-between">
          <span class="text-gray-400">Connected Network:</span>
          <span class="text-white">Ethereum Mainnet</span>
        </div>
        <div class="flex justify-between">
          <span class="text-gray-400">Chain ID:</span>
          <span class="text-white">1</span>
        </div>
        <div class="flex justify-between">
          <span class="text-gray-400">Block Explorer:</span>
          <a href="https://etherscan.io" target="_blank" class="text-yellow-400 hover:text-yellow-300">
            etherscan.io
          </a>
        </div>
      </div>
    </Card>
  {/if}
</div>