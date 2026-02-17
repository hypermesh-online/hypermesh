<script>
  import { 
    wallet_connected,
    token_balances,
    caesar_token_data,
    formatted_caesar_balance,
    demurrage_warning,
    loading_states
  } from '../../stores/wallet.js';
  import { url } from '@roxi/routify';
  import Card from '../ui/Card.svelte';
  import Button from '../ui/Button.svelte';
  import { 
    Send,
    Download,
    TrendingUp,
    AlertTriangle,
    Coins,
    Activity,
    Clock,
    Shield
  } from 'lucide-svelte';
  
  function formatCurrency(amount, currency = 'CAESAR') {
    const num = parseFloat(amount || '0');
    return `${num.toFixed(2)} ${currency}`;
  }
  
  function formatTimestamp(timestamp) {
    return new Date(timestamp).toLocaleDateString();
  }
  
  function calculateDemurrageWarning(caesarData) {
    const daysSinceActivity = (Date.now() - caesarData.last_activity) / (1000 * 60 * 60 * 24);
    return daysSinceActivity > 30; // Warning if inactive for 30+ days
  }
</script>

<div class="space-y-6">
  {#if !$wallet_connected}
    <!-- Wallet Not Connected State -->
    <div class="text-center py-12">
      <div class="w-16 h-16 bg-gradient-to-r from-yellow-400 to-yellow-600 rounded-full flex items-center justify-center mx-auto mb-4">
        <Shield class="w-8 h-8 text-black" />
      </div>
      <h2 class="text-2xl font-bold text-white mb-2">Connect Your Wallet</h2>
      <p class="text-gray-400 mb-6">Connect your wallet to start managing Caesar tokens with anti-speculation mechanics</p>
      <Button size="lg">
        Connect Wallet
      </Button>
    </div>
  {:else}
    <!-- Main Dashboard -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <!-- Caesar Token Balance Card -->
      <Card class="lg:col-span-2">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center space-x-3">
            <div class="w-12 h-12 bg-gradient-to-r from-yellow-400 to-yellow-600 rounded-full flex items-center justify-center">
              <Coins class="w-6 h-6 text-black" />
            </div>
            <div>
              <h3 class="text-lg font-semibold text-white">Caesar Token Balance</h3>
              <p class="text-sm text-gray-400">Anti-speculation currency with demurrage</p>
            </div>
          </div>
          {#if $demurrage_warning || calculateDemurrageWarning($caesar_token_data)}
            <AlertTriangle class="w-5 h-5 text-amber-400" />
          {/if}
        </div>
        
        <div class="space-y-4">
          <div class="flex items-end space-x-4">
            <div>
              <p class="text-sm text-gray-400">Effective Balance</p>
              <p class="text-3xl font-bold caesar-gradient">
                {formatCurrency($caesar_token_data.effective_balance)}
              </p>
            </div>
            {#if $caesar_token_data.accumulated_demurrage > 0}
              <div>
                <p class="text-sm text-red-400">Demurrage Applied</p>
                <p class="text-lg text-red-400">
                  -{formatCurrency($caesar_token_data.accumulated_demurrage.toFixed(2))}
                </p>
              </div>
            {/if}
          </div>
          
          <div class="flex items-center space-x-6 text-sm text-gray-400">
            <div class="flex items-center space-x-2">
              <TrendingUp class="w-4 h-4" />
              <span>Demurrage: {($caesar_token_data.demurrage_rate * 100).toFixed(1)}% annually</span>
            </div>
            <div class="flex items-center space-x-2">
              <Clock class="w-4 h-4" />
              <span>Last Activity: {formatTimestamp($caesar_token_data.last_activity)}</span>
            </div>
          </div>
        </div>
      </Card>

      <!-- Quick Actions -->
      <Card>
        <h3 class="text-lg font-semibold text-white mb-4">Quick Actions</h3>
        <div class="space-y-3">
          <a href={$url('/send')} class="w-full">
            <Button variant="default" class="w-full justify-start">
              <Send class="w-4 h-4 mr-2" />
              Send Tokens
            </Button>
          </a>
          <a href={$url('/receive')} class="w-full">
            <Button variant="outline" class="w-full justify-start">
              <Download class="w-4 h-4 mr-2" />
              Receive Tokens
            </Button>
          </a>
          <a href={$url('/history')} class="w-full">
            <Button variant="ghost" class="w-full justify-start">
              <Activity class="w-4 h-4 mr-2" />
              View History
            </Button>
          </a>
        </div>
      </Card>
    </div>

    <!-- Token Portfolio -->
    <Card>
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold text-white">Token Portfolio</h3>
        <a href={$url('/tokens')}>
          <Button variant="ghost" size="sm">
            Manage Tokens
          </Button>
        </a>
      </div>
      
      <div class="space-y-3">
        {#each Array.from($token_balances.entries()) as [token, balance]}
          <div class="flex items-center justify-between p-3 bg-gray-800 rounded-lg">
            <div class="flex items-center space-x-3">
              <div class="w-8 h-8 bg-gradient-to-r from-yellow-400 to-yellow-600 rounded-full flex items-center justify-center">
                <span class="text-xs font-bold text-black">{token.charAt(0)}</span>
              </div>
              <div>
                <p class="font-medium text-white">{token}</p>
                <p class="text-sm text-gray-400">
                  {#if token === 'CAESAR'}
                    Anti-speculation token
                  {:else}
                    {token} Network
                  {/if}
                </p>
              </div>
            </div>
            <div class="text-right">
              <p class="font-semibold text-white">{formatCurrency(balance, token)}</p>
              {#if token === 'CAESAR'}
                <p class="text-xs text-gray-400">Earning demurrage resistance</p>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </Card>

    <!-- Demurrage Information -->
    {#if $demurrage_warning || calculateDemurrageWarning($caesar_token_data)}
      <Card class="border-amber-500/50 bg-amber-500/5">
        <div class="flex items-start space-x-3">
          <AlertTriangle class="w-5 h-5 text-amber-400 mt-0.5" />
          <div>
            <h4 class="font-semibold text-amber-400 mb-1">Demurrage Warning</h4>
            <p class="text-sm text-gray-300 mb-3">
              Your Caesar tokens have accumulated demurrage due to inactivity. 
              Use your tokens regularly to minimize demurrage effects and maintain their value.
            </p>
            <Button variant="outline" size="sm">
              Learn About Demurrage
            </Button>
          </div>
        </div>
      </Card>
    {/if}
  {/if}
</div>