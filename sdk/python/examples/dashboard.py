"""Dashboard, config, asset, and topology operations.

Run: python examples/dashboard.py
"""

from hypermesh import HyperMeshClient


def main() -> None:
    client = HyperMeshClient()

    # Dashboard info
    info = client.dashboard.info()
    print(f"Dashboard: {info.name} v{info.version} (scope: {info.scope})")

    # List dashboards
    dashboards = client.dashboard.list()
    print(f"\nDashboards: {dashboards.count}")
    for d in dashboards.dashboards:
        print(f"  {d.name} (scope: {d.scope}, url: {d.url})")

    # Show full config
    config = client.config.show()
    print(f"\nNode config: {config}")

    # Get specific config value
    privacy = client.config.get("privacy_mode")
    print(f"privacy_mode: {privacy}")

    # Asset listing
    assets = client.asset.list()
    print(f"\nAssets: {assets.count}")
    for a in assets.assets:
        print(f"  [{a.asset_type}] {a.asset_id} (state: {a.state})")

    # Topology
    topo = client.topology.info()
    print(f"\nTopology:")
    print(f"  Node: {topo.node_id}")
    print(f"  Position: {topo.coordinate}")

    neighbors = client.topology.neighbors()
    print(f"  Neighbors: {neighbors.count} (radius: {neighbors.radius})")
    for n in neighbors.neighbors:
        print(f"    {n.node_id} distance={n.distance}")


if __name__ == "__main__":
    main()
