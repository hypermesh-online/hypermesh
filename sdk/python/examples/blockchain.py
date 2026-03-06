"""Blockchain operations with the HyperMesh Python SDK.

Run: python examples/blockchain.py
"""

from hypermesh import HyperMeshClient


def main() -> None:
    client = HyperMeshClient()

    # Get chain height
    h = client.blockchain.height()
    print(f"Blockchain height: {h.height}")

    # Fetch the genesis block
    genesis = client.blockchain.block(0)
    print(f"\nGenesis block:")
    print(f"  Index: {genesis.index}")
    print(f"  Hash: {genesis.hash}")
    print(f"  Previous hash: {genesis.previous_hash}")
    print(f"  Timestamp: {genesis.timestamp}")
    print(f"  Data: {genesis.data}")

    # Fetch the latest block
    if h.height > 0:
        latest = client.blockchain.block(h.height - 1)
        print(f"\nLatest block (index {latest.index}):")
        print(f"  Hash: {latest.hash}")

    # Validate the chain
    result = client.blockchain.validate()
    print(f"\nBlockchain valid: {result.valid}")
    if result.errors:
        print(f"Errors: {result.errors}")


if __name__ == "__main__":
    main()
