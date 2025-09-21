# Caesar Token Deployment Guide

## 🚀 Quick Start

### Prerequisites
- Node.js v18+ and npm
- Hardhat installed (`npm install`)
- Test ETH on target networks
- Environment variables configured (`.env` file)

### Local Deployment
```bash
# Start local Hardhat node
npx hardhat node

# Deploy to local network
npm run deploy:local
```

### Testnet Deployment

#### 1. Fund Your Deployer Account
The deployer address is: `0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331`

Get test ETH from faucets:
- **Sepolia ETH**: https://sepoliafaucet.com
- **Arbitrum Sepolia**: https://faucet.quicknode.com/arbitrum/sepolia
- **Optimism Sepolia**: https://faucet.quicknode.com/optimism/sepolia
- **Base Sepolia**: https://faucet.quicknode.com/base/sepolia
- **Polygon Mumbai**: https://faucet.polygon.technology
- **BSC Testnet**: https://testnet.binance.org/faucet-smart

#### 2. Deploy to Sepolia
```bash
npm run deploy:sepolia
```

#### 3. Deploy to All Testnets
```bash
npx hardhat run scripts/deploy-all-testnets.ts --network all
```

## 📋 Deployment Checklist

### Phase 1: Local Testing ✅
- [x] Compile all contracts
- [x] Deploy to local Hardhat node
- [x] Verify basic functionality
- [x] Test transfer operations

### Phase 2: Testnet Deployment
- [ ] Fund deployer with Sepolia ETH (0.1 ETH needed)
- [ ] Deploy to Ethereum Sepolia
- [ ] Verify on Etherscan
- [ ] Deploy to Arbitrum Sepolia
- [ ] Deploy to Optimism Sepolia
- [ ] Deploy to Base Sepolia
- [ ] Deploy to Polygon Mumbai
- [ ] Deploy to BSC Testnet

### Phase 3: LayerZero Configuration
- [ ] Set trusted paths between chains
- [ ] Configure peer contracts
- [ ] Set gas limits for cross-chain messages
- [ ] Test cross-chain transfers

### Phase 4: Economic System Testing
- [ ] Test demurrage calculations
- [ ] Verify anti-speculation penalties
- [ ] Test stability mechanisms
- [ ] Validate fiat gateway integration

## 🔧 Contract Architecture

### Main Contracts
1. **CaesarCoin**: Main OFT token with LayerZero V2
   - Creates internal DemurrageManager
   - Creates internal AntiSpeculationEngine
   - Handles cross-chain messaging

2. **Internal Components**:
   - **DemurrageManager**: Time-decay mechanism
   - **AntiSpeculationEngine**: Trading penalties

3. **Supporting Contracts**:
   - **MathUtils**: Mathematical operations library

## 🌐 Network Configuration

| Network | Chain ID | LayerZero Chain ID | Endpoint Address |
|---------|----------|-------------------|------------------|
| Ethereum Sepolia | 11155111 | 40161 | 0x6EDCE65403992e310A62460808c4b910D972f10f |
| Arbitrum Sepolia | 421614 | 40231 | 0x6EDCE65403992e310A62460808c4b910D972f10f |
| Optimism Sepolia | 11155420 | 40232 | 0x6EDCE65403992e310A62460808c4b910D972f10f |
| Base Sepolia | 84532 | 40245 | 0x6EDCE65403992e310A62460808c4b910D972f10f |
| Polygon Mumbai | 80001 | 40109 | 0x6EDCE65403992e310A62460808c4b910D972f10f |
| BSC Testnet | 97 | 40102 | 0x6EDCE65403992e310A62460808c4b910D972f10f |

## 📝 Environment Variables

Create a `.env` file with:
```env
# Wallet Configuration
MNEMONIC="your twelve word mnemonic phrase here"

# RPC URLs
ETH_TESTNET_RPC="https://sepolia.infura.io/v3/YOUR_INFURA_KEY"

# API Keys for Verification
ETHERSCAN_API_KEY="your_etherscan_api_key"
ARBISCAN_API_KEY="your_arbiscan_api_key"
OPTIMISM_API_KEY="your_optimism_api_key"
BASESCAN_API_KEY="your_basescan_api_key"
POLYGONSCAN_API_KEY="your_polygonscan_api_key"
BSCSCAN_API_KEY="your_bscscan_api_key"
```

## 🧪 Testing Cross-Chain Functionality

### 1. Deploy to Multiple Chains
```bash
# Deploy to Sepolia
npm run deploy:sepolia

# Deploy to Arbitrum Sepolia
npx hardhat run scripts/deploy-all-testnets.ts --network arbitrumSepolia
```

### 2. Configure Trusted Paths
After deployment, configure LayerZero trusted paths between chains using the deployment addresses.

### 3. Test Transfer
Use the deployed contracts to test cross-chain transfers between networks.

## 📊 Deployment Status

### Local Network ✅
- **Status**: Deployed Successfully
- **CaesarCoin**: 0xaFC8847AAdf364caDf261E24cc47ACB8D0DE9E24
- **Block**: Latest
- **Functionality**: Verified

### Sepolia Testnet ⏳
- **Status**: Awaiting Funding
- **Required**: 0.1 ETH to deployer account
- **Deployer**: 0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331

### Other Testnets ⏳
- Awaiting Sepolia deployment completion

## 🛠️ Troubleshooting

### Common Issues

1. **Insufficient Balance Error**
   - Ensure deployer has at least 0.1 ETH on target network
   - Check faucet links above for test tokens

2. **Contract Size Too Large**
   - Already optimized with 200 optimizer runs
   - All contracts within size limits

3. **LayerZero Endpoint Not Found**
   - Verify endpoint addresses match network
   - Check LayerZero V2 documentation for updates

4. **Transaction Timeout**
   - Increase gas price in hardhat.config.ts
   - Check network congestion

## 📚 Additional Resources

- [LayerZero V2 Documentation](https://docs.layerzero.network/v2)
- [Hardhat Documentation](https://hardhat.org/docs)
- [OpenZeppelin Contracts](https://docs.openzeppelin.com/contracts)
- [Caesar Token Whitepaper](./whitepaper.md)

## 🔒 Security Considerations

1. **Testnet Only**: Current deployment is for testnet only
2. **Audit Required**: Full security audit needed before mainnet
3. **Private Keys**: Never commit private keys or mnemonics
4. **Access Control**: Verify owner addresses before deployment

## 📞 Support

For deployment issues or questions:
1. Check the [troubleshooting section](#-troubleshooting)
2. Review deployment logs in `deployments/` directory
3. Consult the technical documentation

## ✅ Next Steps

1. **Immediate**: Fund deployer account with Sepolia ETH
2. **Deploy**: Run `npm run deploy:sepolia`
3. **Verify**: Check Etherscan for contract verification
4. **Configure**: Set up LayerZero trusted paths
5. **Test**: Execute cross-chain transfers
6. **Expand**: Deploy to remaining testnets