// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import { OFT } from "@layerzerolabs/oft-evm/contracts/OFT.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title Simple CAES Token
 * @dev Simple LayerZero V2 OFT implementation for migration purposes
 * This is a temporary token for migration - full functionality will be added later
 */
contract SimpleCAES is OFT {
    // Migration tracking
    address public migrationContract;
    bool public migrationEnabled;
    
    // Events
    event MigrationContractSet(address indexed migrationContract);
    event MigrationEnabled(bool enabled);
    
    /**
     * @dev Constructor
     * @param lzEndpoint LayerZero endpoint address
     * @param owner Contract owner
     */
    constructor(
        address lzEndpoint,
        address owner
    ) OFT("CAESAR", "CAES", lzEndpoint, owner) Ownable(owner) {
        // Initialize with zero supply - will be minted during migration
    }
    
    /**
     * @dev Set migration contract address
     */
    function setMigrationContract(address _migrationContract) external onlyOwner {
        require(_migrationContract != address(0), "Invalid migration contract");
        migrationContract = _migrationContract;
        emit MigrationContractSet(_migrationContract);
    }
    
    /**
     * @dev Enable/disable migration
     */
    function setMigrationEnabled(bool _enabled) external onlyOwner {
        migrationEnabled = _enabled;
        emit MigrationEnabled(_enabled);
    }
    
    /**
     * @dev Mint tokens during migration (only callable by migration contract)
     */
    function migrationMint(address to, uint256 amount) external {
        require(msg.sender == migrationContract, "Only migration contract");
        require(migrationEnabled, "Migration not enabled");
        _mint(to, amount);
    }
    
    /**
     * @dev Set minter permissions (for compatibility with TokenMigration)
     */
    function setMinter(address minter, bool canMint) external onlyOwner {
        if (canMint) {
            migrationContract = minter;
            migrationEnabled = true;
        } else {
            if (migrationContract == minter) {
                migrationEnabled = false;
            }
        }
    }
}