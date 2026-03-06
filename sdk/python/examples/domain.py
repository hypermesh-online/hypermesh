"""Domain operations with the HyperMesh Python SDK.

Run: python examples/domain.py
"""

from hypermesh import HyperMeshClient


def main() -> None:
    client = HyperMeshClient()

    # List registered domains
    domains = client.domain.list()
    print(f"Domains: {domains.count}")
    for d in domains.domains:
        print(f"  {d.name} (privacy: {d.privacy}, owner: {d.owner})")

    # Register a new domain
    print("\nRegistering domain 'testapp' with Private privacy...")
    resp = client.domain.register("testapp", "Private")
    print(f"Register result: {resp}")

    # Join a domain
    print("\nJoining domain 'testapp'...")
    join = client.domain.join("testapp")
    print(f"Join result: {join}")


if __name__ == "__main__":
    main()
