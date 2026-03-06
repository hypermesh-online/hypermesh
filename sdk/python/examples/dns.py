"""DNS operations with the HyperMesh Python SDK.

Run: python examples/dns.py
"""

from hypermesh import HyperMeshClient, NotFoundError


def main() -> None:
    client = HyperMeshClient()

    # List existing DNS records
    dns = client.dns.list()
    print(f"DNS records: {dns.count}")
    for record in dns.records:
        print(f"  {record.name} -> {record.address}")

    # Register a new record
    print("\nRegistering example.hypermesh -> ::1")
    resp = client.dns.register("example.hypermesh", "::1")
    print(f"Register result: {resp}")

    # Resolve the record
    try:
        resolved = client.dns.resolve("example.hypermesh")
        print(f"Resolved: {resolved.name} -> {resolved.address}")
    except NotFoundError:
        print("Record not found (may not be registered yet)")

    # List records after registration
    updated = client.dns.list()
    print(f"\nDNS records after registration: {updated.count}")
    for record in updated.records:
        print(f"  {record.name} -> {record.address}")


if __name__ == "__main__":
    main()
