// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers, Contract, Wallet } from 'ethers';
import { config } from '@/config';
import { Logger } from '@/utils/Logger';
import { 
  ChainConfig, 
  ReserveStatus, 
  PegStatus, 
  ReserveDiscrepancy,
  GatewayError,
  ErrorCodes 
} from '@/types';
import { DecimalJS } from 'decimal.js';

// LayerZero OFT ABI (simplified)
const OFT_ABI = [
  'function mint(address to, uint256 amount) external',
  'function burn(uint256 amount) external',
  'function totalSupply() external view returns (uint256)',
  'function balanceOf(address account) external view returns (uint256)',
  'function send(address to, uint256 amount, uint32 dstEid, bytes memory options, bytes memory composeMsg) payable external',
  'function quote(uint32 dstEid, bytes memory message, bytes memory options, bool payInLzToken) external view returns (uint256 nativeFee, uint256 lzTokenFee)',
  'event Transfer(address indexed from, address indexed to, uint256 value)',
  'event OFTSent(bytes32 indexed guid, uint32 dstEid, address indexed toAddress, uint256 amountSentLD, uint256 amountReceivedLD)',
];

// USDC ABI (simplified)
const USDC_ABI = [
  'function balanceOf(address account) external view returns (uint256)',
  'function transfer(address to, uint256 amount) external returns (bool)',
  'function approve(address spender, uint256 amount) external returns (bool)',
  'function decimals() external view returns (uint8)',
];

export class LayerZeroService {
  private logger: Logger;
  private providers: Map<number, ethers.JsonRpcProvider>;
  private gateContracts: Map<number, Contract>;
  private usdcContracts: Map<number, Contract>;
  private wallet: Wallet;

  constructor() {
    this.logger = new Logger('LayerZeroService');
    this.providers = new Map();
    this.gateContracts = new Map();
    this.usdcContracts = new Map();
    
    // Initialize wallet
    this.wallet = new Wallet(config.blockchain.gatewayOperatorKey);
    
    // Initialize providers and contracts for each supported chain
    this.initializeChainConnections();
  }

  /**
   * Initialize connections to all supported chains
   */
  private initializeChainConnections(): void {
    config.layerZero.supportedChains.forEach(chain => {
      try {
        // Initialize provider
        const provider = new ethers.JsonRpcProvider(chain.rpcUrl);
        this.providers.set(chain.chainId, provider);

        // Connect wallet to provider
        const walletWithProvider = this.wallet.connect(provider);

        // Initialize CAESAR token contract
        const gateContract = new Contract(
          config.layerZero.gateTokenAddress,
          OFT_ABI,
          walletWithProvider
        );
        this.gateContracts.set(chain.chainId, gateContract);

        // Initialize USDC contract
        const usdcContract = new Contract(
          config.layerZero.usdcAddress,
          USDC_ABI,
          walletWithProvider
        );
        this.usdcContracts.set(chain.chainId, usdcContract);

        this.logger.info('Initialized chain connection', { 
          chainId: chain.chainId, 
          name: chain.name 
        });
      } catch (error) {
        this.logger.error('Failed to initialize chain connection', { 
          chainId: chain.chainId, 
          name: chain.name, 
          error 
        });
      }
    });
  }

  /**
   * Mint CAESAR tokens after successful fiat deposit
   */
  async mintGATETokens(params: {
    userId: string;
    amount: string;
    destinationChain: number;
    destinationAddress: string;
    stripePaymentIntentId: string;
  }): Promise<{
    txHash: string;
    blockNumber: number;
    gasUsed: string;
    totalSupplyAfter: string;
  }> {
    try {
      this.logger.info('Minting CAESAR tokens', params);

      const gateContract = this.gateContracts.get(params.destinationChain);
      const provider = this.providers.get(params.destinationChain);

      if (!gateContract || !provider) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          `Unsupported destination chain: ${params.destinationChain}`,
          400
        );
      }

      // Validate mint amount
      const mintAmount = new DecimalJS(params.amount);
      if (mintAmount.isZero() || mintAmount.isNegative()) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Invalid mint amount',
          400
        );
      }

      // Get current gas price with multiplier
      const feeData = await provider.getFeeData();
      const gasPrice = feeData.gasPrice! * BigInt(
        Math.floor(this.getGasMultiplier(params.destinationChain) * 100)
      ) / BigInt(100);

      // Estimate gas limit
      const gasLimit = await gateContract.mint.estimateGas(
        params.destinationAddress,
        params.amount
      );

      // Execute mint transaction
      const tx = await gateContract.mint(
        params.destinationAddress,
        params.amount,
        {
          gasPrice,
          gasLimit: gasLimit * BigInt(120) / BigInt(100), // 20% buffer
        }
      );

      this.logger.info('Mint transaction submitted', { 
        txHash: tx.hash,
        chainId: params.destinationChain 
      });

      // Wait for confirmation
      const receipt = await tx.wait(
        this.getRequiredConfirmations(params.destinationChain)
      );

      if (!receipt || receipt.status !== 1) {
        throw new GatewayError(
          ErrorCodes.TRANSACTION_FAILED,
          'Mint transaction failed',
          500
        );
      }

      // Get updated total supply
      const totalSupplyAfter = await gateContract.totalSupply();

      this.logger.info('CAESAR tokens minted successfully', {
        txHash: receipt.hash,
        blockNumber: receipt.blockNumber,
        gasUsed: receipt.gasUsed.toString(),
        totalSupplyAfter: totalSupplyAfter.toString()
      });

      return {
        txHash: receipt.hash,
        blockNumber: receipt.blockNumber,
        gasUsed: receipt.gasUsed.toString(),
        totalSupplyAfter: totalSupplyAfter.toString(),
      };

    } catch (error) {
      this.logger.error('Failed to mint CAESAR tokens', { error, params });
      throw this.handleBlockchainError(error);
    }
  }

  /**
   * Burn CAESAR tokens for fiat withdrawal
   */
  async burnGATETokens(params: {
    userId: string;
    amount: string;
    sourceChain: number;
    userAddress: string;
    stripeTransferId: string;
  }): Promise<{
    txHash: string;
    blockNumber: number;
    gasUsed: string;
    totalSupplyAfter: string;
  }> {
    try {
      this.logger.info('Burning CAESAR tokens', params);

      const gateContract = this.gateContracts.get(params.sourceChain);
      const provider = this.providers.get(params.sourceChain);

      if (!gateContract || !provider) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          `Unsupported source chain: ${params.sourceChain}`,
          400
        );
      }

      // Validate burn amount
      const burnAmount = new DecimalJS(params.amount);
      if (burnAmount.isZero() || burnAmount.isNegative()) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Invalid burn amount',
          400
        );
      }

      // Check user balance
      const userBalance = await gateContract.balanceOf(params.userAddress);
      if (new DecimalJS(userBalance.toString()).lessThan(burnAmount)) {
        throw new GatewayError(
          ErrorCodes.INSUFFICIENT_FUNDS,
          'Insufficient CAESAR token balance',
          400
        );
      }

      // Get current gas price with multiplier
      const feeData = await provider.getFeeData();
      const gasPrice = feeData.gasPrice! * BigInt(
        Math.floor(this.getGasMultiplier(params.sourceChain) * 100)
      ) / BigInt(100);

      // Note: In a real implementation, this would require the user to approve
      // and initiate the burn transaction, not the gateway operator
      // This is simplified for demonstration purposes

      // Estimate gas limit
      const gasLimit = await gateContract.burn.estimateGas(params.amount);

      // Execute burn transaction
      const tx = await gateContract.burn(params.amount, {
        gasPrice,
        gasLimit: gasLimit * BigInt(120) / BigInt(100), // 20% buffer
      });

      this.logger.info('Burn transaction submitted', { 
        txHash: tx.hash,
        chainId: params.sourceChain 
      });

      // Wait for confirmation
      const receipt = await tx.wait(
        this.getRequiredConfirmations(params.sourceChain)
      );

      if (!receipt || receipt.status !== 1) {
        throw new GatewayError(
          ErrorCodes.TRANSACTION_FAILED,
          'Burn transaction failed',
          500
        );
      }

      // Get updated total supply
      const totalSupplyAfter = await gateContract.totalSupply();

      this.logger.info('CAESAR tokens burned successfully', {
        txHash: receipt.hash,
        blockNumber: receipt.blockNumber,
        gasUsed: receipt.gasUsed.toString(),
        totalSupplyAfter: totalSupplyAfter.toString()
      });

      return {
        txHash: receipt.hash,
        blockNumber: receipt.blockNumber,
        gasUsed: receipt.gasUsed.toString(),
        totalSupplyAfter: totalSupplyAfter.toString(),
      };

    } catch (error) {
      this.logger.error('Failed to burn CAESAR tokens', { error, params });
      throw this.handleBlockchainError(error);
    }
  }

  /**
   * Cross-chain bridge CAESAR tokens using LayerZero
   */
  async bridgeGATETokens(params: {
    fromChain: number;
    toChain: number;
    amount: string;
    recipient: string;
    sender: string;
  }): Promise<{
    txHash: string;
    layerZeroGuid: string;
    estimatedArrivalTime: number;
  }> {
    try {
      this.logger.info('Bridging CAESAR tokens', params);

      const sourceContract = this.gateContracts.get(params.fromChain);
      const sourceProvider = this.providers.get(params.fromChain);
      const targetChain = config.layerZero.supportedChains.find(c => c.chainId === params.toChain);

      if (!sourceContract || !sourceProvider || !targetChain) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Unsupported chain configuration',
          400
        );
      }

      // Get LayerZero endpoint ID for destination chain
      const dstEid = targetChain.layerZeroChainId;

      // Prepare LayerZero options (simplified)
      const options = ethers.solidityPacked(
        ['uint16', 'uint256'],
        [1, config.blockchain.gasLimit.crossChain]
      );

      // Get quote for LayerZero fees
      const [nativeFee] = await sourceContract.quote(
        dstEid,
        ethers.toUtf8Bytes(''), // empty message for simple transfer
        options,
        false // pay in native token
      );

      this.logger.info('LayerZero fee quote', {
        nativeFee: nativeFee.toString(),
        dstEid,
        fromChain: params.fromChain,
        toChain: params.toChain
      });

      // Execute cross-chain send
      const tx = await sourceContract.send(
        params.recipient,
        params.amount,
        dstEid,
        options,
        ethers.toUtf8Bytes(''), // empty compose message
        {
          value: nativeFee, // pay LayerZero fees
          gasLimit: config.blockchain.gasLimit.crossChain,
        }
      );

      const receipt = await tx.wait(
        this.getRequiredConfirmations(params.fromChain)
      );

      if (!receipt || receipt.status !== 1) {
        throw new GatewayError(
          ErrorCodes.TRANSACTION_FAILED,
          'Bridge transaction failed',
          500
        );
      }

      // Extract LayerZero GUID from logs (simplified)
      const layerZeroGuid = this.extractLayerZeroGuid(receipt);

      // Estimate arrival time based on destination chain
      const estimatedArrivalTime = this.estimateBridgeTime(params.fromChain, params.toChain);

      this.logger.info('Bridge transaction successful', {
        txHash: receipt.hash,
        layerZeroGuid,
        estimatedArrivalTime
      });

      return {
        txHash: receipt.hash,
        layerZeroGuid,
        estimatedArrivalTime,
      };

    } catch (error) {
      this.logger.error('Failed to bridge CAESAR tokens', { error, params });
      throw this.handleBlockchainError(error);
    }
  }

  /**
   * Sync reserves across all chains
   */
  async syncReserves(): Promise<ReserveStatus> {
    try {
      this.logger.info('Syncing reserves across all chains');

      const reserveData = await Promise.all(
        config.layerZero.supportedChains.map(async (chain) => {
          try {
            const gateContract = this.gateContracts.get(chain.chainId);
            const usdcContract = this.usdcContracts.get(chain.chainId);

            if (!gateContract || !usdcContract) {
              return null;
            }

            const [gateSupply, usdcBalance] = await Promise.all([
              gateContract.totalSupply(),
              usdcContract.balanceOf(config.layerZero.gateTokenAddress), // USDC held by GATE contract
            ]);

            return {
              chainId: chain.chainId,
              chainName: chain.name,
              gateSupply: gateSupply.toString(),
              usdcReserve: usdcBalance.toString(),
            };
          } catch (error) {
            this.logger.error(`Failed to sync reserves for chain ${chain.chainId}`, { error });
            return null;
          }
        })
      );

      // Filter out failed requests
      const validReserves = reserveData.filter(data => data !== null);

      // Calculate totals
      let totalGATESupply = new DecimalJS(0);
      let totalUSDCReserves = new DecimalJS(0);
      const discrepancies: ReserveDiscrepancy[] = [];

      validReserves.forEach(reserve => {
        if (reserve) {
          const gateSupply = new DecimalJS(reserve.gateSupply).div(1e18); // Convert from wei
          const usdcReserve = new DecimalJS(reserve.usdcReserve).div(1e6); // USDC has 6 decimals

          totalGATESupply = totalGATESupply.plus(gateSupply);
          totalUSDCReserves = totalUSDCReserves.plus(usdcReserve);

          // Check for discrepancies (GATE should be 1:1 backed by USDC)
          const difference = gateSupply.minus(usdcReserve).abs();
          if (difference.greaterThan(0.01)) { // Allow 1 cent tolerance
            const severity = difference.greaterThan(1000) ? 'CRITICAL' :
                           difference.greaterThan(100) ? 'HIGH' :
                           difference.greaterThan(10) ? 'MEDIUM' : 'LOW';

            discrepancies.push({
              chain: reserve.chainId,
              expectedBalance: gateSupply.toString(),
              actualBalance: usdcReserve.toString(),
              difference: difference.toString(),
              severity: severity as any,
            });
          }
        }
      });

      const pegRatio = totalUSDCReserves.div(totalGATESupply).toNumber();
      const reserveHealthScore = this.calculateReserveHealthScore(pegRatio, discrepancies);

      const reserveStatus: ReserveStatus = {
        totalUSDCReserves: totalUSDCReserves.toString(),
        totalGATESupply: totalGATESupply.toString(),
        pegRatio,
        reserveHealthScore,
        lastSyncTime: new Date(),
        discrepancies,
      };

      this.logger.info('Reserve sync completed', {
        totalGATESupply: totalGATESupply.toString(),
        totalUSDCReserves: totalUSDCReserves.toString(),
        pegRatio,
        discrepancyCount: discrepancies.length
      });

      return reserveStatus;

    } catch (error) {
      this.logger.error('Failed to sync reserves', { error });
      throw this.handleBlockchainError(error);
    }
  }

  /**
   * Validate peg maintenance
   */
  async validatePegMaintenance(): Promise<PegStatus> {
    const reserves = await this.syncReserves();
    const targetPeg = 1.0;
    const pegTolerance = 0.02; // 2% tolerance

    const deviation = Math.abs(reserves.pegRatio - targetPeg);
    const isWithinThreshold = deviation <= pegTolerance;

    const correctionActions: string[] = [];
    if (!isWithinThreshold) {
      if (reserves.pegRatio > targetPeg) {
        correctionActions.push('Increase USDC reserves');
        correctionActions.push('Reduce GATE minting');
      } else {
        correctionActions.push('Burn excess CAESAR tokens');
        correctionActions.push('Add USDC to reserves');
      }
    }

    return {
      currentPeg: reserves.pegRatio,
      targetPeg,
      deviation,
      isWithinThreshold,
      correctionActions,
      lastCorrection: new Date(), // Would track actual correction times
    };
  }

  /**
   * Get real-time gas estimates for all chains
   */
  async getGasEstimates(): Promise<Record<number, {
    gasPrice: string;
    estimatedCostUSD: number;
  }>> {
    const estimates: Record<number, any> = {};

    await Promise.all(
      config.layerZero.supportedChains.map(async (chain) => {
        try {
          const provider = this.providers.get(chain.chainId);
          if (provider) {
            const feeData = await provider.getFeeData();
            const gasPrice = feeData.gasPrice?.toString() || '0';
            
            // Estimate cost in USD (simplified - would use real price feeds)
            const estimatedCostUSD = this.estimateGasCostUSD(chain.chainId, gasPrice);
            
            estimates[chain.chainId] = {
              gasPrice,
              estimatedCostUSD,
            };
          }
        } catch (error) {
          this.logger.error(`Failed to get gas estimate for chain ${chain.chainId}`, { error });
        }
      })
    );

    return estimates;
  }

  /**
   * Helper methods
   */
  private getGasMultiplier(chainId: number): number {
    const chain = config.layerZero.supportedChains.find(c => c.chainId === chainId);
    return chain?.gasMultiplier || 1.2;
  }

  private getRequiredConfirmations(chainId: number): number {
    const chain = config.layerZero.supportedChains.find(c => c.chainId === chainId);
    return chain?.confirmations || 12;
  }

  private extractLayerZeroGuid(receipt: ethers.TransactionReceipt): string {
    // This would parse the actual logs to extract the GUID
    // Simplified for demonstration
    return `lz_${receipt.hash.slice(0, 10)}`;
  }

  private estimateBridgeTime(fromChain: number, toChain: number): number {
    // Estimate based on chain characteristics
    // Ethereum: 15 minutes, L2s: 5 minutes, etc.
    const baseTimes: Record<number, number> = {
      1: 900,    // Ethereum: 15 minutes
      137: 300,  // Polygon: 5 minutes
      42161: 300, // Arbitrum: 5 minutes
      10: 300,   // Optimism: 5 minutes
      8453: 300, // Base: 5 minutes
    };

    const fromTime = baseTimes[fromChain] || 600;
    const toTime = baseTimes[toChain] || 600;
    
    return Math.max(fromTime, toTime);
  }

  private calculateReserveHealthScore(pegRatio: number, discrepancies: ReserveDiscrepancy[]): number {
    let score = 100;

    // Deduct points for peg deviation
    const pegDeviation = Math.abs(pegRatio - 1.0);
    score -= pegDeviation * 1000; // 10 points per 1% deviation

    // Deduct points for discrepancies
    discrepancies.forEach(discrepancy => {
      switch (discrepancy.severity) {
        case 'LOW': score -= 1; break;
        case 'MEDIUM': score -= 5; break;
        case 'HIGH': score -= 15; break;
        case 'CRITICAL': score -= 50; break;
      }
    });

    return Math.max(0, score);
  }

  private estimateGasCostUSD(chainId: number, gasPrice: string): number {
    // Simplified gas cost estimation in USD
    // Would use real price feeds in production
    const gasPriceGwei = new DecimalJS(gasPrice).div(1e9);
    const gasLimit = 200000; // Average gas limit
    const gasCost = gasPriceGwei.mul(gasLimit).div(1e9); // ETH cost

    // Rough ETH price (would be dynamic)
    const ethPriceUSD = 2000;
    return gasCost.mul(ethPriceUSD).toNumber();
  }

  private handleBlockchainError(error: any): GatewayError {
    if (error.code === 'INSUFFICIENT_FUNDS') {
      return new GatewayError(
        ErrorCodes.INSUFFICIENT_FUNDS,
        'Insufficient funds for transaction',
        400
      );
    }
    
    if (error.code === 'UNPREDICTABLE_GAS_LIMIT') {
      return new GatewayError(
        ErrorCodes.TRANSACTION_FAILED,
        'Transaction would fail',
        400
      );
    }

    return new GatewayError(
      ErrorCodes.SYSTEM_ERROR,
      `Blockchain error: ${error.message}`,
      500
    );
  }
}