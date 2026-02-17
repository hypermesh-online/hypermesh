// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from 'hardhat';
import { SignerWithAddress } from '@nomicfoundation/hardhat-ethers/signers';
import {
  CaesarToken,
  SimpleEconomicEngine,
  AdvancedDemurrageManager,
  AdvancedAntiSpeculationEngine,
  StabilityPool,
  CrossChainEconomicSync,
  StripeIntegrationManager,
  MockERC20,
  MockPriceOracle,
  MockStripeOracle,
  MockAMM,
  MockLayerZeroEndpoint,
  MockLZEndpointV2,
  MockChainlinkOracle,
  MockTimelock
} from '../../typechain-types';

export interface TestFixtureResult {
  // Core contracts
  caesarToken: CaesarToken;
  economicEngine: SimpleEconomicEngine;
  demurrageManager: AdvancedDemurrageManager;
  antiSpeculationEngine: AdvancedAntiSpeculationEngine;
  stabilityPool: StabilityPool;
  crossChainSync: CrossChainEconomicSync;
  stripeIntegration: StripeIntegrationManager;
  
  // Mock contracts
  mockUSDC: MockERC20;
  mockCSR: MockERC20;
  priceOracle: MockPriceOracle;
  stripeOracle: MockStripeOracle;
  mockAMM: MockAMM;
  mockLZEndpoint: MockLayerZeroEndpoint;
  mockLZEndpointV2: MockLZEndpointV2;
  chainlinkOracle: MockChainlinkOracle;
  timelock: MockTimelock;
  
  // Signers
  owner: SignerWithAddress;
  user1: SignerWithAddress;
  user2: SignerWithAddress;
  user3: SignerWithAddress;
  fiatOracle: SignerWithAddress;
  emergencyOperator: SignerWithAddress;
  keeper: SignerWithAddress;
  
  // Constants
  TARGET_PRICE: bigint;
  PRECISION: bigint;
  BASIS_POINTS: bigint;
  INITIAL_SUPPLY: bigint;
}

/**
 * Complete Caesar test fixture
 * Deploys all contracts and sets up initial state for comprehensive testing
 */
export async function deployCaesarFixture(): Promise<TestFixtureResult> {
  const [owner, user1, user2, user3, fiatOracle, emergencyOperator, keeper] = 
    await ethers.getSigners();
  
  // Constants
  const TARGET_PRICE = ethers.parseEther('1'); // 1 USD
  const PRECISION = ethers.parseEther('1');
  const BASIS_POINTS = 10000n;
  const INITIAL_SUPPLY = ethers.parseEther('1000000'); // 1M tokens
  
  // Deploy mock dependencies first
  console.log('Deploying mock contracts...');
  
  // Mock tokens
  const MockERC20Factory = await ethers.getContractFactory('MockERC20');
  const mockUSDC = await MockERC20Factory.deploy('Mock USDC', 'USDC', 6);
  const mockCSR = await MockERC20Factory.deploy('Mock Caesar', 'CAES', 18);
  
  // Mock oracles
  const MockPriceOracleFactory = await ethers.getContractFactory('MockPriceOracle');
  const priceOracle = await MockPriceOracleFactory.deploy(TARGET_PRICE);
  
  const MockStripeOracleFactory = await ethers.getContractFactory('MockStripeOracle');
  const stripeOracle = await MockStripeOracleFactory.deploy();
  
  const MockChainlinkOracleFactory = await ethers.getContractFactory('MockChainlinkOracle');
  const chainlinkOracle = await MockChainlinkOracleFactory.deploy(100000000); // $1 in 8 decimals
  
  // Mock AMM
  const MockAMMFactory = await ethers.getContractFactory('MockAMM');
  const mockAMM = await MockAMMFactory.deploy(
    await mockCSR.getAddress(),
    await mockUSDC.getAddress()
  );
  
  // Mock LayerZero endpoints
  const MockLZEndpointFactory = await ethers.getContractFactory('MockLayerZeroEndpoint');
  const mockLZEndpoint = await MockLZEndpointFactory.deploy();
  
  const MockLZEndpointV2Factory = await ethers.getContractFactory('MockLZEndpointV2');
  const mockLZEndpointV2 = await MockLZEndpointV2Factory.deploy();
  
  // Mock timelock
  const MockTimelockFactory = await ethers.getContractFactory('MockTimelock');
  const timelock = await MockTimelockFactory.deploy();
  
  console.log('Deploying core contracts...');
  
  // Deploy core economic components
  const AdvancedDemurrageManagerFactory = await ethers.getContractFactory('AdvancedDemurrageManager');
  const demurrageManager = await AdvancedDemurrageManagerFactory.deploy(
    owner.address,
    fiatOracle.address
  );
  
  const AdvancedAntiSpeculationEngineFactory = await ethers.getContractFactory('AdvancedAntiSpeculationEngine');
  const antiSpeculationEngine = await AdvancedAntiSpeculationEngineFactory.deploy(
    owner.address
  );
  
  const StabilityPoolFactory = await ethers.getContractFactory('StabilityPool');
  const stabilityPool = await StabilityPoolFactory.deploy(
    owner.address,
    await mockCSR.getAddress(),
    await mockUSDC.getAddress()
  );
  
  const SimpleEconomicEngineFactory = await ethers.getContractFactory('SimpleEconomicEngine');
  const economicEngine = await SimpleEconomicEngineFactory.deploy(
    owner.address,
    await demurrageManager.getAddress(),
    await antiSpeculationEngine.getAddress(),
    await stabilityPool.getAddress()
  );
  
  const CrossChainEconomicSyncFactory = await ethers.getContractFactory('CrossChainEconomicSync');
  const crossChainSync = await CrossChainEconomicSyncFactory.deploy(
    await mockLZEndpointV2.getAddress(),
    owner.address,
    await economicEngine.getAddress()
  );
  
  const StripeIntegrationManagerFactory = await ethers.getContractFactory('StripeIntegrationManager');
  const stripeIntegration = await StripeIntegrationManagerFactory.deploy(
    owner.address,
    await economicEngine.getAddress(),
    await stripeOracle.getAddress()
  );
  
  console.log('Deploying main CaesarToken contract...');
  
  // Deploy main CaesarToken contract (updated from Caesar)
  const CaesarTokenFactory = await ethers.getContractFactory('CaesarToken');
  const caesarToken = await CaesarTokenFactory.deploy(
    await mockLZEndpointV2.getAddress(),
    owner.address
  );
  
  console.log('Setting up initial state and permissions...');
  
  // Setup initial token balances for testing
  await mockUSDC.mint(owner.address, ethers.parseUnits('10000000', 6)); // 10M USDC
  await mockUSDC.mint(user1.address, ethers.parseUnits('100000', 6)); // 100K USDC
  await mockUSDC.mint(user2.address, ethers.parseUnits('50000', 6)); // 50K USDC
  
  // Setup AMM liquidity
  await mockUSDC.approve(await mockAMM.getAddress(), ethers.parseUnits('1000000', 6));
  await mockCSR.mint(owner.address, ethers.parseEther('1000000'));
  await mockCSR.approve(await mockAMM.getAddress(), ethers.parseEther('1000000'));
  await mockAMM.addLiquidity(
    ethers.parseEther('1000000'), // 1M CSR
    ethers.parseUnits('1000000', 6) // 1M USDC
  );
  
  // Setup permissions and roles
  await economicEngine.addEmergencyOperator(emergencyOperator.address);
  // Note: StabilityPool uses Ownable, not role-based access control
  
  // Configure initial parameters
  const initialParams = {
    baseDemurrageRate: 50n, // 0.5% hourly
    maxDemurrageRate: 200n, // 2% hourly
    stabilityThreshold: 100n, // 1%
    fiatDiscountFactor: 5000n, // 50% max discount
    gracePeriodsHours: 48n, // 48 hours
    interventionThreshold: 500n, // 5%
    rebalanceFrequency: 3600n, // 1 hour
    emergencyThreshold: 1000n // 10%
  };
  
  await economicEngine.updateEconomicParameters(initialParams);
  
  console.log('Test fixture setup complete');
  
  return {
    // Core contracts
    caesarToken,
    economicEngine,
    demurrageManager,
    antiSpeculationEngine,
    stabilityPool,
    crossChainSync,
    stripeIntegration,
    
    // Mock contracts
    mockUSDC,
    mockCSR,
    priceOracle,
    stripeOracle,
    mockAMM,
    mockLZEndpoint,
    mockLZEndpointV2,
    chainlinkOracle,
    timelock,
    
    // Signers
    owner,
    user1,
    user2,
    user3,
    fiatOracle,
    emergencyOperator,
    keeper,
    
    // Constants
    TARGET_PRICE,
    PRECISION,
    BASIS_POINTS,
    INITIAL_SUPPLY
  };
}


/**
 * Lightweight fixture for basic unit tests
 */
export async function deployBasicFixture() {
  const [owner, user1, user2] = await ethers.getSigners();
  
  const MockLZEndpointV2Factory = await ethers.getContractFactory('MockLZEndpointV2');
  const mockLZEndpoint = await MockLZEndpointV2Factory.deploy();
  
  const CaesarTokenFactory = await ethers.getContractFactory('CaesarToken');
  const caesarToken = await CaesarTokenFactory.deploy(
    await mockLZEndpoint.getAddress(),
    owner.address
  );
  
  // Setup migration for testing
  await caesarToken.setMigrationContract(owner.address);
  await caesarToken.setMigrationEnabled(true);
  
  // Mint initial tokens to owner for testing
  const initialSupply = ethers.parseEther('1000000'); // 1M tokens
  await caesarToken.migrationMint(owner.address, initialSupply);
  
  return {
    caesarToken,
    mockLZEndpoint,
    owner,
    user1,
    user2,
    initialSupply
  };
}

/**
 * Fixture for stress testing with multiple users and large balances
 */
export async function deployStressTestFixture() {
  const signers = await ethers.getSigners();
  const [owner, ...users] = signers;
  
  const fixture = await deployCaesarFixture();
  
  // Create additional test users with various balance tiers
  const testUsers = users.slice(0, 20); // Use first 20 signers as test users
  
  // Distribute tokens across user tiers
  for (let i = 0; i < testUsers.length; i++) {
    const user = testUsers[i];
    const tierMultiplier = Math.floor(i / 5) + 1; // Groups of 5 users per tier
    const baseAmount = ethers.parseEther('1000'); // 1K base amount
    const userAmount = baseAmount * BigInt(tierMultiplier);
    
    await fixture.caesarToken.transfer(user.address, userAmount);
    await fixture.mockUSDC.mint(user.address, ethers.parseUnits((Number(userAmount) / 1e18).toString(), 6));
  }
  
  return {
    ...fixture,
    testUsers
  };
}

/**
 * Fixture for cross-chain testing
 */
export async function deployCrossChainFixture() {
  const fixture = await deployCaesarFixture();
  
  // Deploy additional CaesarToken instances for different chains
  const chains = [
    { id: 1, name: 'Ethereum' },
    { id: 137, name: 'Polygon' },
    { id: 42161, name: 'Arbitrum' },
    { id: 10, name: 'Optimism' }
  ];
  
  const chainInstances = [];
  
  for (const chain of chains) {
    const MockLZEndpointV2Factory = await ethers.getContractFactory('MockLZEndpointV2');
    const endpoint = await MockLZEndpointV2Factory.deploy();
    
    const CaesarTokenFactory = await ethers.getContractFactory('CaesarToken');
    const instance = await CaesarTokenFactory.deploy(
      await endpoint.getAddress(),
      fixture.owner.address
    );
    
    chainInstances.push({
      chainId: chain.id,
      name: chain.name,
      contract: instance,
      endpoint
    });
  }
  
  return {
    ...fixture,
    chainInstances
  };
}

/**
 * Fixture for performance testing
 */
export async function deployPerformanceTestFixture() {
  const fixture = await deployCaesarFixture();
  
  // Pre-populate with transaction history
  const accounts = [fixture.user1, fixture.user2, fixture.user3];
  const baseAmount = ethers.parseEther('100');
  
  // Create transaction history for realistic testing
  for (let i = 0; i < 50; i++) {
    const fromUser = accounts[i % accounts.length];
    const toUser = accounts[(i + 1) % accounts.length];
    
    await fixture.caesarToken.connect(fromUser).transfer(
      toUser.address,
      baseAmount + BigInt(i) * ethers.parseEther('10')
    );
    
    // Add some time variance
    if (i % 5 === 0) {
      await ethers.provider.send('evm_increaseTime', [3600]); // 1 hour
      await ethers.provider.send('evm_mine', []);
    }
  }
  
  return fixture;
}