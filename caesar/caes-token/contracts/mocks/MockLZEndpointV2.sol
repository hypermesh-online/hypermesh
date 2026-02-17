// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import { ILayerZeroEndpointV2 } from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import { MessagingParams, MessagingFee } from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";

/**
 * @title MockLZEndpointV2
 * @dev Mock LayerZero V2 endpoint for testing
 */
contract MockLZEndpointV2 {
    uint32 public constant eid = 1; // Mock endpoint ID
    
    event PacketSent(bytes encodedPacket, bytes options, address sendLibrary);
    
    function send(
        MessagingParams calldata /*_params*/,
        address /*_refundAddress*/
    ) external payable returns (MessagingFee memory fee) {
        // Mock implementation - return basic fee structure
        return MessagingFee({
            nativeFee: msg.value > 0 ? msg.value : 0.001 ether,
            lzTokenFee: 0
        });
    }
    
    function quote(
        MessagingParams calldata /*_params*/,
        address /*_sender*/
    ) external pure returns (MessagingFee memory fee) {
        return MessagingFee({
            nativeFee: 0.001 ether,
            lzTokenFee: 0
        });
    }
    
    function clear(
        address /*_oapp*/,
        Origin calldata /*_origin*/,
        bytes32 /*_guid*/,
        bytes calldata /*_message*/
    ) external {
        // Mock implementation
    }
    
    function setLzToken(address /*_lzToken*/) external {
        // Mock implementation
    }
    
    function lzToken() external pure returns (address) {
        return address(0);
    }
    
    function nativeToken() external pure returns (address) {
        return address(0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE);
    }
    
    function setDelegate(address /*_delegate*/) external {
        // Mock implementation
    }
    
    function delegates(address /*_oapp*/) external pure returns (address) {
        return address(0);
    }
    
    function initialize(
        address /*_delegate*/,
        address /*_sendLibs*/,
        address /*_receiveLibs*/
    ) external {
        // Mock implementation
    }
    
    function registerLibrary(
        address /*_lib*/
    ) external {
        // Mock implementation
    }
    
    function isRegisteredLibrary(
        address /*_lib*/
    ) external pure returns (bool) {
        return true;
    }
    
    function getRegisteredLibraries() external pure returns (address[] memory) {
        return new address[](0);
    }
    
    function setSendLibrary(
        address /*_oapp*/,
        uint32 /*_eid*/,
        address /*_sendLib*/
    ) external {
        // Mock implementation
    }
    
    function setReceiveLibrary(
        address /*_oapp*/,
        uint32 /*_eid*/,
        address /*_receiveLib*/,
        uint256 /*_expiry*/
    ) external {
        // Mock implementation
    }
    
    function setReceiveLibraryTimeout(
        address /*_oapp*/,
        uint32 /*_eid*/,
        address /*_lib*/,
        uint256 /*_expiry*/
    ) external {
        // Mock implementation
    }
    
    function receiveLibraryTimeout(
        address /*_oapp*/,
        uint32 /*_eid*/
    ) external pure returns (address lib, uint256 expiry) {
        return (address(0), 0);
    }
    
    function getSendLibrary(
        address /*_oapp*/,
        uint32 /*_eid*/
    ) external pure returns (address lib) {
        return address(0);
    }
    
    function getReceiveLibrary(
        address /*_oapp*/,
        uint32 /*_eid*/
    ) external pure returns (address lib, bool isDefault) {
        return (address(0), true);
    }
    
    function isDefaultSendLibrary(
        address /*_oapp*/,
        uint32 /*_eid*/
    ) external pure returns (bool) {
        return true;
    }
    
    function setConfig(
        address /*_oapp*/,
        address /*_lib*/,
        SetConfigParam[] calldata /*_params*/
    ) external {
        // Mock implementation
    }
    
    function getConfig(
        address /*_oapp*/,
        address /*_lib*/,
        uint32 /*_eid*/,
        uint32 /*_configType*/
    ) external pure returns (bytes memory config) {
        return "";
    }
}

// Required structs for interface compatibility
struct Origin {
    uint32 srcEid;
    bytes32 sender;
    uint64 nonce;
}

struct SetConfigParam {
    uint32 eid;
    uint32 configType;
    bytes config;
}