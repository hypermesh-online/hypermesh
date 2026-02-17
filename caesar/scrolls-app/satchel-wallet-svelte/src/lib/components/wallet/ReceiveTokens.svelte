<script>
  import { wallet_connected, wallet_address } from '../../stores/wallet.js';
  import Card from '../ui/Card.svelte';
  import Button from '../ui/Button.svelte';
  import { 
    Download,
    ArrowLeft,
    Copy,
    QrCode,
    CheckCircle
  } from 'lucide-svelte';
  import { url } from '@roxi/routify';
  
  let copied = false;
  
  function copyAddress() {
    if ($wallet_address) {
      navigator.clipboard.writeText($wallet_address);
      copied = true;
      setTimeout(() => copied = false, 2000);
    }
  }
  
  // TODO: Generate QR code for address
  function generateQRCode(address) {
    return `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(address)}`;
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
      <h1 class="text-2xl font-bold text-white">Receive Tokens</h1>
      <p class="text-gray-400">Share your address to receive tokens</p>
    </div>
  </div>

  {#if !$wallet_connected}
    <Card class="text-center py-8">
      <p class="text-gray-400">Please connect your wallet to receive tokens</p>
    </Card>
  {:else}
    <!-- Address Card -->
    <Card>
      <div class="text-center">
        <div class="w-16 h-16 bg-gradient-to-r from-yellow-400 to-yellow-600 rounded-full flex items-center justify-center mx-auto mb-4">
          <Download class="w-8 h-8 text-black" />
        </div>
        
        <h3 class="text-lg font-semibold text-white mb-2">Your Wallet Address</h3>
        <p class="text-gray-400 mb-6">Share this address to receive tokens from others</p>
        
        <!-- QR Code -->
        <div class="bg-white rounded-lg p-4 inline-block mb-6">
          <img 
            src={generateQRCode($wallet_address)} 
            alt="Wallet address QR code"
            class="w-48 h-48"
          />
        </div>
        
        <!-- Address Display -->
        <div class="bg-gray-800 rounded-lg p-4 mb-4">
          <p class="font-mono text-sm text-white break-all">
            {$wallet_address}
          </p>
        </div>
        
        <!-- Copy Button -->
        <Button 
          variant="default" 
          on:click={copyAddress}
          class="w-full"
        >
          {#if copied}
            <CheckCircle class="w-4 h-4 mr-2" />
            Address Copied!
          {:else}
            <Copy class="w-4 h-4 mr-2" />
            Copy Address
          {/if}
        </Button>
      </div>
    </Card>

    <!-- Instructions -->
    <Card>
      <h3 class="text-lg font-semibold text-white mb-4">How to Receive Tokens</h3>
      <div class="space-y-3 text-sm text-gray-300">
        <div class="flex items-start space-x-3">
          <div class="w-6 h-6 bg-yellow-400 rounded-full flex items-center justify-center text-black font-bold text-xs mt-0.5">
            1
          </div>
          <p>Share your wallet address or QR code with the sender</p>
        </div>
        <div class="flex items-start space-x-3">
          <div class="w-6 h-6 bg-yellow-400 rounded-full flex items-center justify-center text-black font-bold text-xs mt-0.5">
            2
          </div>
          <p>Wait for the sender to initiate the transaction</p>
        </div>
        <div class="flex items-start space-x-3">
          <div class="w-6 h-6 bg-yellow-400 rounded-full flex items-center justify-center text-black font-bold text-xs mt-0.5">
            3
          </div>
          <p>Tokens will appear in your wallet once the transaction is confirmed</p>
        </div>
      </div>
    </Card>

    <!-- Caesar Token Information -->
    <Card class="border-yellow-500/30 bg-yellow-500/5">
      <h3 class="text-lg font-semibold text-yellow-400 mb-3">Caesar Token Benefits</h3>
      <div class="space-y-2 text-sm text-gray-300">
        <p>• Anti-speculation mechanics prevent harmful price speculation</p>
        <p>• Demurrage system encourages economic circulation</p>
        <p>• Designed for real economic activity, not speculation</p>
        <p>• Regular usage helps maintain token value over time</p>
      </div>
    </Card>

    <!-- Network Information -->
    <Card>
      <h3 class="text-lg font-semibold text-white mb-3">Important Notes</h3>
      <div class="space-y-2 text-sm text-gray-400">
        <p>• Make sure the sender is using the correct network</p>
        <p>• Double-check the address before sharing</p>
        <p>• Transactions may take a few minutes to confirm</p>
        <p>• You can track incoming transactions in your history</p>
      </div>
    </Card>
  {/if}
</div>