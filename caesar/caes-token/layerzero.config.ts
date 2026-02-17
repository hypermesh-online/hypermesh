// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { EndpointId } from '@layerzerolabs/lz-definitions';

import type { OAppOmniGraphHardhat, OmniPointHardhat } from '@layerzerolabs/toolbox-hardhat';

const sepoliaContract: OmniPointHardhat = {
    eid: EndpointId.SEPOLIA_V2_TESTNET,
    contractName: 'CaesarCoin',
};

const mumbaiContract: OmniPointHardhat = {
    eid: EndpointId.POLYGON_V2_TESTNET,
    contractName: 'CaesarCoin',
};

const bscTestnetContract: OmniPointHardhat = {
    eid: EndpointId.BSC_V2_TESTNET,
    contractName: 'CaesarCoin',
};

const arbitrumSepoliaContract: OmniPointHardhat = {
    eid: EndpointId.ARBITRUM_V2_TESTNET,
    contractName: 'CaesarCoin',
};

const optimismSepoliaContract: OmniPointHardhat = {
    eid: EndpointId.OPTIMISM_V2_TESTNET,
    contractName: 'CaesarCoin',
};

const baseSepoliaContract: OmniPointHardhat = {
    eid: EndpointId.BASE_V2_TESTNET,
    contractName: 'CaesarCoin',
};

const config: OAppOmniGraphHardhat = {
    contracts: [
        {
            contract: sepoliaContract,
        },
        {
            contract: mumbaiContract,
        },
        {
            contract: bscTestnetContract,
        },
        {
            contract: arbitrumSepoliaContract,
        },
        {
            contract: optimismSepoliaContract,
        },
        {
            contract: baseSepoliaContract,
        },
    ],
    connections: [
        // Sepolia connections
        {
            from: sepoliaContract,
            to: mumbaiContract,
        },
        {
            from: sepoliaContract,
            to: bscTestnetContract,
        },
        {
            from: sepoliaContract,
            to: arbitrumSepoliaContract,
        },
        {
            from: sepoliaContract,
            to: optimismSepoliaContract,
        },
        {
            from: sepoliaContract,
            to: baseSepoliaContract,
        },
        // Mumbai connections
        {
            from: mumbaiContract,
            to: sepoliaContract,
        },
        {
            from: mumbaiContract,
            to: bscTestnetContract,
        },
        {
            from: mumbaiContract,
            to: arbitrumSepoliaContract,
        },
        {
            from: mumbaiContract,
            to: optimismSepoliaContract,
        },
        {
            from: mumbaiContract,
            to: baseSepoliaContract,
        },
        // BSC Testnet connections
        {
            from: bscTestnetContract,
            to: sepoliaContract,
        },
        {
            from: bscTestnetContract,
            to: mumbaiContract,
        },
        {
            from: bscTestnetContract,
            to: arbitrumSepoliaContract,
        },
        {
            from: bscTestnetContract,
            to: optimismSepoliaContract,
        },
        {
            from: bscTestnetContract,
            to: baseSepoliaContract,
        },
        // Arbitrum Sepolia connections
        {
            from: arbitrumSepoliaContract,
            to: sepoliaContract,
        },
        {
            from: arbitrumSepoliaContract,
            to: mumbaiContract,
        },
        {
            from: arbitrumSepoliaContract,
            to: bscTestnetContract,
        },
        {
            from: arbitrumSepoliaContract,
            to: optimismSepoliaContract,
        },
        {
            from: arbitrumSepoliaContract,
            to: baseSepoliaContract,
        },
        // Optimism Sepolia connections
        {
            from: optimismSepoliaContract,
            to: sepoliaContract,
        },
        {
            from: optimismSepoliaContract,
            to: mumbaiContract,
        },
        {
            from: optimismSepoliaContract,
            to: bscTestnetContract,
        },
        {
            from: optimismSepoliaContract,
            to: arbitrumSepoliaContract,
        },
        {
            from: optimismSepoliaContract,
            to: baseSepoliaContract,
        },
        // Base Sepolia connections
        {
            from: baseSepoliaContract,
            to: sepoliaContract,
        },
        {
            from: baseSepoliaContract,
            to: mumbaiContract,
        },
        {
            from: baseSepoliaContract,
            to: bscTestnetContract,
        },
        {
            from: baseSepoliaContract,
            to: arbitrumSepoliaContract,
        },
        {
            from: baseSepoliaContract,
            to: optimismSepoliaContract,
        },
    ],
};

export default config;