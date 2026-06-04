# Substrate (`base`) — Implementation Contract

> Canonical architecture: **`papers/SUBSTRATE.md`** (the authority). This document is
> the engineer-facing contract: trait shapes, the adapter registry, the address
> mapping, and the exact STOQ integration targets. Where the two differ, the
> whitepaper wins.

## What this crate is

The **Substrate** is the network layer beneath the kernel. It produces the address,
interface, and reachability that STOQ today assumes already exist. Requirements:
**R15** (sovereign self-assigned addressing) and **R16** (link-sovereign interface
management), `papers/HYPERMESH.md` §3.

**Current state: scaffold.** Traits and types are stable and compile; backend bodies
are `todo!()` (real backend) or return `SubstrateError::Unsupported` (stubs).

## Layering (hard rule)

- `base` depends ONLY on `hypermesh-lib` (+ leaf deps). Verify with
  `cargo tree -p base` — it must show no `stoq`, `trustchain`, or `blockmatrix`.
- STOQ does NOT depend on `base`. The **node binary** constructs the `Substrate` and
  injects resolved values into `TransportConfig`. The Substrate stays strictly below
  transport.
- Address derivation consumes `hypermesh_lib::NodeId`, NOT `trustchain::FalconIdentity`
  (avoids a cycle).

## Traits (`src/substrate.rs`)

```rust
trait Substrate {
    async fn local_address(&self, node_id: &NodeId) -> SubstrateResult<Ipv6Addr>; // R15
    async fn reachability(&self) -> SubstrateResult<Reachability>;                 // R15
    async fn active_interface(&self) -> SubstrateResult<InterfaceId>;              // R16
    async fn watch_links(&self) -> SubstrateResult<BoxStream<'static, LinkEvent>>; // R16
}

trait SubstrateAdapter {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> SubstrateCapabilities;
    async fn enumerate_interfaces(&self) -> SubstrateResult<Vec<InterfaceId>>;
    async fn carrier_state(&self, iface: &InterfaceId) -> SubstrateResult<LinkState>;
    async fn assign_address(&self, iface: &InterfaceId, addr: InterfaceAddress) -> SubstrateResult<()>;
    async fn detect_reachability(&self) -> SubstrateResult<Reachability>;
}
```

`SubstrateCapabilities { enumerate, carrier, assign_address, watch, reachability }` —
each adapter advertises what it can do; the registry selects the highest-capability
backend that satisfies a predicate (`src/adapters/mod.rs`).

## Address mapping (`src/address.rs`, R15)

```
  bits   0..32   fd48:4d00          fixed ULA prefix (HYPERMESH_PREFIX, R1)
  bits  32..64   <subnet>           SUBNET_DEVICE_SCOPE = 0 when unjoined
  bits  64..128  node_id[24..32]    low 8 bytes of the 32-byte NodeId digest
```

`derive_address(node_id, subnet) -> Ipv6Addr` and
`verify_address(addr, node_id, subnet) -> bool`. Pure functions of the FALCON public
key (via `NodeId::from_public_key`), so independently verifiable. **Bodies are
`todo!()` until Phase 1** — the mapping is the stable contract.

## Adapters (`src/adapters/`)

| Module | Capabilities | Status |
|---|---|---|
| `rtnetlink_linux` (feature `rtnetlink-backend`) | all (netlink) | bodies `todo!()`, Phase 1/2 |
| `sysfs_fallback` | enumerate + carrier (read-only) | bodies `todo!()`, Phase 2 |
| `radio_mesh` | none (Substrate.c R&D) | stub, `Unsupported` |
| `windows` | none (future) | stub, `Unsupported` |

`SubstrateAdapterRegistry::with_defaults()` registers them most-capable-first;
`select(predicate)` / `enumerator()` / `address_assigner()` pick a backend.

## STOQ integration targets (Phase 1 — NOT changed in scaffold)

| STOQ location | After |
|---|---|
| `stoq/src/transport/config.rs:109` `bind_address` | `Substrate::local_address(node_id)` |
| `stoq/src/transport/config.rs:169` `public_ipv6` | `Substrate::reachability()` |
| `stoq/src/transport/config.rs:159` `ebpf_interface` | `Substrate::active_interface()` |
| `stoq/src/transport/manager/constructors.rs:36-47` `detect_outbound_interface()` | replaced by `Substrate::active_interface()` |

Wiring happens at the node binary (`blockmatrix/src/bin/node/`), which constructs the
Substrate and passes resolved values into `TransportConfig`.

## Features

- `default = []` — scaffold builds with no netlink deps on any platform.
- `rtnetlink-backend` — compiles `rtnetlink_linux`, pulls `rtnetlink` +
  `netlink-packet-route`. **Before enabling in Phase 1: verify musl-buildability**
  (deploy target builds musl static-pie — `core/CLAUDE.md`).

## Build / verify

```
cargo build -p base                          # scaffold, default features
cargo build -p base --features rtnetlink-backend
cargo tree -p base | grep -E 'stoq|trustchain|blockmatrix'   # must be empty
```
