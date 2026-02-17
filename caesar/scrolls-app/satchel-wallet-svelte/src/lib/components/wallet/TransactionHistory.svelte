<script>
  import { 
    wallet_connected,
    transaction_history,
    loading_states
  } from '../../stores/wallet.js';
  import Card from '../ui/Card.svelte';
  import Button from '../ui/Button.svelte';
  import { 
    History,
    ArrowLeft,
    ArrowUpRight,
    ArrowDownLeft,
    ExternalLink,
    Filter,
    RefreshCw
  } from 'lucide-svelte';
  import { url } from '@roxi/routify';
  import { walletStore } from '../../stores/wallet.js';
  
  let filterType = 'all'; // all, sent, received
  let filterToken = 'all';
  
  $: filteredTransactions = $transaction_history.filter(tx => {
    const matchesType = filterType === 'all' || tx.type === filterType;
    const matchesToken = filterToken === 'all' || tx.token === filterToken;
    return matchesType && matchesToken;
  });
  
  $: uniqueTokens = [...new Set($transaction_history.map(tx => tx.token))];
  
  function formatTimestamp(timestamp) {
    const date = new Date(timestamp);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString();
  }
  
  function truncateHash(hash) {
    return `${hash.slice(0, 10)}...${hash.slice(-8)}`;
  }
  
  function truncateAddress(address) {
    return `${address.slice(0, 6)}...${address.slice(-4)}`;
  }
  
  function getStatusColor(status) {
    switch (status) {
      case 'confirmed': return 'text-green-400';
      case 'pending': return 'text-yellow-400';
      case 'failed': return 'text-red-400';
      default: return 'text-gray-400';
    }
  }
  
  function getTypeIcon(type) {
    return type === 'send' ? ArrowUpRight : ArrowDownLeft;
  }
  
  function getTypeColor(type) {
    return type === 'send' ? 'text-red-400' : 'text-green-400';
  }
  
  function openInExplorer(hash) {
    // TODO: Use actual block explorer based on network
    window.open(`https://etherscan.io/tx/${hash}`, '_blank');
  }
  
  async function refreshHistory() {
    await walletStore.loadTransactionHistory();
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
      <h1 class="text-2xl font-bold text-white">Transaction History</h1>
      <p class="text-gray-400">View all your token transactions</p>
    </div>
    <Button 
      variant="outline" 
      size="sm" 
      on:click={refreshHistory}
      disabled={$loading_states.loading_history}
    >
      {#if $loading_states.loading_history}
        <RefreshCw class="w-4 h-4 mr-2 animate-spin" />
        Loading...
      {:else}
        <RefreshCw class="w-4 h-4 mr-2" />
        Refresh
      {/if}
    </Button>
  </div>

  {#if !$wallet_connected}
    <Card class="text-center py-8">
      <p class="text-gray-400">Please connect your wallet to view transaction history</p>
    </Card>
  {:else}
    <!-- Filters -->
    <Card>
      <div class="flex flex-col md:flex-row md:items-center md:justify-between space-y-4 md:space-y-0">
        <div class="flex items-center space-x-2">
          <Filter class="w-4 h-4 text-gray-400" />
          <span class="text-sm font-medium text-gray-300">Filter:</span>
        </div>
        
        <div class="flex flex-col sm:flex-row space-y-2 sm:space-y-0 sm:space-x-4">
          <!-- Type Filter -->
          <select 
            bind:value={filterType}
            class="bg-gray-800 border border-gray-600 rounded-md px-3 py-1 text-sm text-white focus:border-yellow-400 focus:outline-none"
          >
            <option value="all">All Transactions</option>
            <option value="send">Sent</option>
            <option value="receive">Received</option>
          </select>
          
          <!-- Token Filter -->
          <select 
            bind:value={filterToken}
            class="bg-gray-800 border border-gray-600 rounded-md px-3 py-1 text-sm text-white focus:border-yellow-400 focus:outline-none"
          >
            <option value="all">All Tokens</option>
            {#each uniqueTokens as token}
              <option value={token}>{token}</option>
            {/each}
          </select>
        </div>
      </div>
    </Card>

    <!-- Transaction List -->
    {#if filteredTransactions.length === 0}
      <Card class="text-center py-12">
        <History class="w-12 h-12 text-gray-500 mx-auto mb-4" />
        <h3 class="text-lg font-medium text-gray-400 mb-2">No Transactions Found</h3>
        <p class="text-gray-500">
          {#if $transaction_history.length === 0}
            You haven't made any transactions yet
          {:else}
            No transactions match your current filters
          {/if}
        </p>
      </Card>
    {:else}
      <Card class="p-0">
        <div class="divide-y divide-gray-700">
          {#each filteredTransactions as transaction}
            <div class="p-6 hover:bg-gray-800/50 transition-colors">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-4">
                  <!-- Transaction Type Icon -->
                  <div class="w-10 h-10 rounded-full bg-gray-800 flex items-center justify-center">
                    <svelte:component this={getTypeIcon(transaction.type)} class="w-5 h-5 {getTypeColor(transaction.type)}" />
                  </div>
                  
                  <!-- Transaction Details -->
                  <div>
                    <div class="flex items-center space-x-2 mb-1">
                      <span class="font-medium text-white capitalize">
                        {transaction.type === 'send' ? 'Sent' : 'Received'} {transaction.token}
                      </span>
                      <span class="text-xs px-2 py-1 rounded-full {getStatusColor(transaction.status)} bg-gray-800 capitalize">
                        {transaction.status}
                      </span>
                    </div>
                    <div class="flex items-center space-x-4 text-sm text-gray-400">
                      <span>
                        {transaction.type === 'send' ? 'To' : 'From'}: 
                        {truncateAddress(transaction.type === 'send' ? transaction.to : transaction.from)}
                      </span>
                      <span>{formatTimestamp(transaction.timestamp)}</span>
                    </div>
                  </div>
                </div>
                
                <!-- Amount and Actions -->
                <div class="flex items-center space-x-4">
                  <div class="text-right">
                    <p class="font-medium {getTypeColor(transaction.type)}">
                      {transaction.type === 'send' ? '-' : '+'}{transaction.value} {transaction.token}
                    </p>
                    <p class="text-xs text-gray-400">
                      {truncateHash(transaction.hash)}
                    </p>
                  </div>
                  
                  <Button 
                    variant="ghost" 
                    size="icon" 
                    on:click={() => openInExplorer(transaction.hash)}
                    title="View on block explorer"
                  >
                    <ExternalLink class="w-4 h-4" />
                  </Button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </Card>
    {/if}

    <!-- Load More (Future Enhancement) -->
    {#if filteredTransactions.length > 0}
      <div class="text-center">
        <Button variant="outline" disabled>
          Load More Transactions
        </Button>
        <p class="text-xs text-gray-500 mt-2">
          Showing {filteredTransactions.length} of {$transaction_history.length} transactions
        </p>
      </div>
    {/if}
  {/if}
</div>