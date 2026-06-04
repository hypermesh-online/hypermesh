# Substrate (`base`) Development Context

## Status: Planning (scaffold only)

This crate is the **Substrate** — the network layer *beneath* the kernel. It is
currently a scaffold: trait contracts and types are defined and compile; backend
method bodies are `todo!()` (real backend) or return `Unsupported` (stubs). No
network behavior is implemented yet.

Canonical spec: `papers/SUBSTRATE.md`. Implementation contract: `SPEC.md` (this dir).

## Why this exists

Everything in HyperMesh is defined from the kernel up. The paper calls the OS
kernel "Layer 0" (`papers/HYPERMESH.md` §4). But the kernel itself assumes a live
carrier, a routable IPv6, and a working interface already exist — borrowed from the
incumbent network (DHCP, ISP-assigned addresses, NAT). The Substrate owns that
floor, turning HyperMesh from "a better protocol over the incumbent's link" into "a
network that does not need the incumbent."

R1 already mandates nodes carry `fd48:4d00::/32` addresses; the Substrate is the
producer that was missing. R15/R16 (`papers/HYPERMESH.md` §3) make it normative.

## Architectural truths

1. **Beneath the kernel, never "Layer 0"** — the kernel already owns that name.
2. **No upward dependency** — `base` depends only on `hypermesh-lib`. STOQ does NOT
   depend on `base`. The node binary injects Substrate-derived values
   (`bind_address`, `public_ipv6`, `ebpf_interface`) into STOQ's `TransportConfig`.
3. **Identity → address** — the address is a pure function of
   `NodeId = BLAKE3(falcon_pubkey)`; any peer can recompute and verify it. No DHCP.
4. **Capability-tier selection** — the registry picks the best backend and degrades
   netlink → sysfs → fallback, mirroring the eBPF tier model.

## Scope (phased)

| Sub-stratum | What | Phase |
|---|---|---|
| Substrate.a | Sovereign addressing + reachability (`address.rs`, `reachability.rs`) | 1 |
| Substrate.b | Link/carrier/interface mgmt (`link.rs`, rtnetlink) | 2 |
| Substrate.c | Physical/radio, zero ISP (`adapters/radio_mesh.rs`) | 3 (R&D stub) |

## Modules

| Module | Purpose |
|--------|---------|
| `substrate.rs` | `Substrate` + `SubstrateAdapter` traits, `SubstrateCapabilities` |
| `address.rs` | Substrate.a: `NodeId` → `fd48:4d00::/32` derivation + verification |
| `reachability.rs` | Substrate.a: `Reachability`, `PathKind` |
| `link.rs` | Substrate.b: `InterfaceId`, `LinkState`, `LinkEvent`, `InterfaceAddress` |
| `error.rs` | `SubstrateError`, `SubstrateResult` |
| `adapters/` | Backend registry + 4 adapters (rtnetlink, sysfs, radio, windows) |

## Features

- `rtnetlink-backend` (off by default) — compiles the real Linux netlink adapter and
  pulls `rtnetlink`/`netlink-packet-route`. Turned on in Phase 1.
  **TODO before Phase 1**: verify these deps build under musl (deploy target builds
  musl static-pie — see `core/CLAUDE.md` build notes).

## STOQ integration targets (NOT changed in the scaffold pass)

- `stoq/src/transport/config.rs` — `bind_address` (:109), `public_ipv6` (:169),
  `ebpf_interface` (:159) become Substrate-injected.
- `stoq/src/transport/manager/constructors.rs` — `detect_outbound_interface()`
  (:36-47) is replaced by `Substrate::active_interface()`.
