// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title Basic CAES Token
 * @dev Basic ERC20 implementation for migration purposes
 * No LayerZero dependencies - pure ERC20
 */
contract BasicCAES is ERC20, Ownable {
    // Migration tracking
    address public migrationContract;
    bool public migrationEnabled;
    
    // Events
    event MigrationContractSet(address indexed migrationContract);
    event MigrationEnabled(bool enabled);
    
    /**
     * @dev Constructor
     * @param owner Contract owner
     */
    constructor(
        address owner
    ) ERC20("CAESAR", "CAES") Ownable(owner) {
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
    
    /**
     * @dev Owner-only mint function for direct migration
     */
    function ownerMint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }
}