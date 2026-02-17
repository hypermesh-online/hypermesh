<script>
  import { 
    wallet_connected,
    token_balances,
    loading_states
  } from '../../stores/wallet.js';
  import Card from '../ui/Card.svelte';
  import Button from '../ui/Button.svelte';
  import Input from '../ui/Input.svelte';
  import { 
    Send,
    ArrowLeft,
    AlertTriangle,
    CheckCircle
  } from 'lucide-svelte';
  import { url } from '@roxi/routify';
  
  let selectedToken = 'CAESAR';
  let recipientAddress = '';
  let amount = '';
  let gasPrice = 'standard';
  let showConfirmation = false;
  let transactionResult = null;
  
  $: availableBalance = $token_balances.get(selectedToken) || '0.00';
  $: isValidAddress = recipientAddress.startsWith('0x') && recipientAddress.length === 42;
  $: isValidAmount = parseFloat(amount) > 0 && parseFloat(amount) <= parseFloat(availableBalance);
  $: canSend = isValidAddress && isValidAmount && selectedToken;
  
  function handleMaxAmount() {
    amount = availableBalance;
  }
  
  function showTransactionConfirmation() {
    if (!canSend) return;
    showConfirmation = true;
  }
  
  async function executeSend() {
    try {
      loading_states.update(state => ({ ...state, sending: true }));
      
      // TODO: Implement actual transaction sending
      // Simulate transaction for now
      await new Promise(resolve => setTimeout(resolve, 3000));
      
      transactionResult = {
        success: true,
        hash: '0x' + Math.random().toString(16).substr(2, 40),
        amount,
        token: selectedToken,
        recipient: recipientAddress
      };
      
      // Reset form
      recipientAddress = '';
      amount = '';
      showConfirmation = false;
      
    } catch (error) {
      transactionResult = {
        success: false,
        error: error.message
      };
    } finally {
      loading_states.update(state => ({ ...state, sending: false }));
    }
  }
  
  function resetForm() {
    transactionResult = null;
    showConfirmation = false;
    recipientAddress = '';
    amount = '';
  }
</script>

<div class="max-w-2xl mx-auto space-y-6">
  <!-- Header -->
  <div class="flex items-center space-x-4">
    <a href={$url('/')}>
      <Button variant="ghost" size="icon">
        <ArrowLeft class="w-4 h-4" />
      </Button>
    </a>
    <div>
      <h1 class="text-2xl font-bold text-white">Send Tokens</h1>
      <p class="text-gray-400">Transfer tokens to another wallet address</p>
    </div>
  </div>

  {#if !$wallet_connected}
    <Card class="text-center py-8">
      <p class="text-gray-400">Please connect your wallet to send tokens</p>
    </Card>
  {:else if transactionResult}
    <!-- Transaction Result -->
    <Card class={transactionResult.success ? "border-green-500/50 bg-green-500/5" : "border-red-500/50 bg-red-500/5"}>
      <div class="flex items-start space-x-3">
        {#if transactionResult.success}
          <CheckCircle class="w-6 h-6 text-green-400 mt-0.5" />
          <div class="flex-1">
            <h3 class="font-semibold text-green-400 mb-2">Transaction Sent Successfully</h3>
            <div class="space-y-2 text-sm text-gray-300">
              <p><span class="text-gray-400">Amount:</span> {transactionResult.amount} {transactionResult.token}</p>
              <p><span class="text-gray-400">To:</span> {transactionResult.recipient}</p>
              <p><span class="text-gray-400">Transaction Hash:</span> {transactionResult.hash}</p>
            </div>
            <div class="flex space-x-3 mt-4">
              <Button variant="outline" size="sm" on:click={resetForm}>
                Send Another
              </Button>
              <a href={$url('/history')}>
                <Button variant="ghost" size="sm">
                  View History
                </Button>
              </a>
            </div>
          </div>
        {:else}
          <AlertTriangle class="w-6 h-6 text-red-400 mt-0.5" />
          <div class="flex-1">
            <h3 class="font-semibold text-red-400 mb-2">Transaction Failed</h3>
            <p class="text-sm text-gray-300 mb-4">{transactionResult.error}</p>
            <Button variant="outline" size="sm" on:click={resetForm}>
              Try Again
            </Button>
          </div>
        {/if}
      </div>
    </Card>
  {:else if showConfirmation}
    <!-- Transaction Confirmation -->
    <Card>
      <h3 class="text-lg font-semibold text-white mb-4">Confirm Transaction</h3>
      <div class="space-y-4 mb-6">
        <div class="bg-gray-800 rounded-lg p-4 space-y-3">
          <div class="flex justify-between">
            <span class="text-gray-400">Token:</span>
            <span class="text-white font-medium">{selectedToken}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-400">Amount:</span>
            <span class="text-white font-medium">{amount} {selectedToken}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-400">To:</span>
            <span class="text-white font-mono text-sm">{recipientAddress}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-400">Gas Price:</span>
            <span class="text-white capitalize">{gasPrice}</span>
          </div>
        </div>
        
        {#if selectedToken === 'CAESAR'}
          <div class="bg-amber-500/10 border border-amber-500/30 rounded-lg p-4">
            <div class="flex items-start space-x-2">
              <AlertTriangle class="w-4 h-4 text-amber-400 mt-0.5" />
              <div class="text-sm">
                <p class="text-amber-400 font-medium">Caesar Token Notice</p>
                <p class="text-gray-300">
                  Remember: Caesar tokens implement demurrage mechanics to prevent speculation. 
                  Regular usage helps maintain token value.
                </p>
              </div>
            </div>
          </div>
        {/if}
      </div>
      
      <div class="flex space-x-3">
        <Button 
          variant="default" 
          on:click={executeSend}
          disabled={$loading_states.sending}
          class="flex-1"
        >
          {#if $loading_states.sending}
            Sending...
          {:else}
            <Send class="w-4 h-4 mr-2" />
            Send Transaction
          {/if}
        </Button>
        <Button 
          variant="outline" 
          on:click={() => showConfirmation = false}
          disabled={$loading_states.sending}
        >
          Cancel
        </Button>
      </div>
    </Card>
  {:else}
    <!-- Send Form -->
    <Card>
      <h3 class="text-lg font-semibold text-white mb-4">Send Transaction</h3>
      
      <div class="space-y-4">
        <!-- Token Selection -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Token</label>
          <select 
            bind:value={selectedToken}
            class="w-full bg-gray-900 border border-gray-600 rounded-md px-3 py-2 text-white focus:border-yellow-400 focus:outline-none"
          >
            {#each Array.from($token_balances.entries()) as [token, balance]}
              <option value={token}>{token} (Balance: {balance})</option>
            {/each}
          </select>
        </div>

        <!-- Recipient Address -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Recipient Address</label>
          <Input
            bind:value={recipientAddress}
            placeholder="0x..."
            class={!isValidAddress && recipientAddress ? "border-red-500" : ""}
          />
          {#if recipientAddress && !isValidAddress}
            <p class="text-red-400 text-sm mt-1">Please enter a valid Ethereum address</p>
          {/if}
        </div>

        <!-- Amount -->
        <div>
          <div class="flex items-center justify-between mb-2">
            <label class="block text-sm font-medium text-gray-300">Amount</label>
            <span class="text-sm text-gray-400">
              Balance: {availableBalance} {selectedToken}
            </span>
          </div>
          <div class="flex space-x-2">
            <Input
              bind:value={amount}
              type="number"
              step="0.01"
              placeholder="0.00"
              class={!isValidAmount && amount ? "border-red-500 flex-1" : "flex-1"}
            />
            <Button variant="outline" size="sm" on:click={handleMaxAmount}>
              Max
            </Button>
          </div>
          {#if amount && !isValidAmount}
            <p class="text-red-400 text-sm mt-1">
              {parseFloat(amount) > parseFloat(availableBalance) ? 'Insufficient balance' : 'Please enter a valid amount'}
            </p>
          {/if}
        </div>

        <!-- Gas Price -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Gas Price</label>
          <div class="grid grid-cols-3 gap-2">
            {#each ['slow', 'standard', 'fast'] as speed}
              <button
                class="px-3 py-2 rounded-md text-sm font-medium transition-colors {gasPrice === speed ? 'bg-yellow-400 text-black' : 'bg-gray-800 text-gray-300 hover:bg-gray-700'}"
                on:click={() => gasPrice = speed}
              >
                {speed.charAt(0).toUpperCase() + speed.slice(1)}
              </button>
            {/each}
          </div>
        </div>
      </div>

      <div class="mt-6">
        <Button 
          variant="default"
          disabled={!canSend}
          on:click={showTransactionConfirmation}
          class="w-full"
        >
          <Send class="w-4 h-4 mr-2" />
          Review Transaction
        </Button>
      </div>
    </Card>
  {/if}
</div>