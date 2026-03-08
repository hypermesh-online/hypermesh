"""Basic usage of the HyperMesh Python SDK.

Run: python examples/basic.py
"""

from hypermesh import HyperMeshClient, HyperMeshError, NotFoundError

def main() -> None:
    client = HyperMeshClient()  # https://localhost:8443

    # Ping the node
    alive = client.node.ping()
    print(f"Node alive: {alive}")

    # Get node status
    status = client.node.status()
    print(f"Node ID: {status.node_id}")
    print(f"Chain height: {status.chain_height}")
    print(f"Peers: {status.peers}")
    print(f"Privacy mode: {status.privacy_mode}")
    print(f"Uptime: {status.uptime_secs} seconds")
    print(f"Coordinate: {status.coordinate}")

    # List connected peers
    peers = client.network.peers()
    print(f"\nConnected peers: {peers.count}")
    for peer in peers.peers:
        print(f"  {peer.node_id} @ {peer.address}")

    # Error handling
    try:
        client.blockchain.block(999999)
    except NotFoundError:
        print("\nExpected: block 999999 not found")
    except HyperMeshError as e:
        print(f"\nError fetching block 999999: {e}")


if __name__ == "__main__":
    main()
