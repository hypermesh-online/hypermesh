<!-- Written by Richard Christopher, Copyright 2026 HyperMesh Foundation -->

# VISION.md — HyperMesh Architecture

The definitive statement of what HyperMesh is, how traffic flows through it, what each crate is
responsible for, and how a mesh network defines itself.

---

## 1. Native vs. Slave Mode

**HyperMesh-Native** is the formal posture: a device is a first-class mesh citizen.

**Slave mode** is a host — `clouds`, or any other OS — running HyperMesh *subordinately*. It is
explicitly **not HyperMesh-Native**.

> The **HyperMesh Foundation will not formally adopt slave mode. CloudsOS will.**

Slave mode is a real, supported integration posture for a host OS, but it lives outside the
Foundation's native specification.

---

## 2. The Path a Packet Takes

**Traffic arrives through eBPF, which is running STOQ at the port.** It leverages **TrustChain**
and **NGauge**.

The eBPF program's job is deliberately narrow. It does **not** move payloads. It transfers:

- **instructions** or **addresses**, and
- **the validation carried by the block itself**

encrypted by that device's **local TrustChain configuration** and the **validity of its individual
blocks**. Types come from `lib`; definitions come from **Catalog**.

This is instruction-based retrieval enforced at the gate: you send where and how to get a thing,
plus the proof it is that thing — never the bulk of the thing itself.

---

## 3. Crate Responsibilities

| Crate | Responsibility |
|---|---|
| **`lib`** | **The types.** Catalog and BlockMatrix both leverage *and implement* it. **All Proof-of-State lives here.** |
| **`stoq`** | The transport — run **at the port by eBPF**. STOQ = Secure Tokenization Over QUIC; PoS *is* the transport. |
| **`hypermesh-ebpf`** | The gate. Runs STOQ at the port; transfers instructions/addresses + block validation. |
| **`trustchain`** | A **decentralized DNS server, via eBPF** — in parallel with, and partially via, NGauge. Also the certificate authority that secures device-to-device sessions. |
| **NGauge** (`engauge`) | **Resource dispatching — and that is all it has to do.** Two faces: an **interface for Catalog**, and a **scheduler for BlockMatrix**. Shard-distributes blocks. |
| **`catalog`** | The **package manager for the shared Asset library** on a network. Holds the definitions. |
| **`blockmatrix`** | The **distributed storage substrate** — where Blocks are distributed across the network. **Secured by TrustChain, shard-distributed by NGauge.** |

---

## 4. Locality, Mirrors, and Why Assets Have Their Own IPs

**An NGauge "network" hosts some locality of some matrices' assets.** Not the whole matrix — a
*locality*: the spatial neighborhood of assets that network is responsible for.

**Everyone hosting those Assets — via their blocks and shards — is acting as a MIRROR.**

This is precisely why **Assets have their own IPs**. The addressable entity is the *asset*, not the
machine. A device is not a destination; it is one of many surfaces reflecting an asset that lives at
its own address. Many mirrors, one asset identity. Devices are traceable *through* the assets they
mirror.

**TrustChain handles the certificates between devices** whenever a new **DNS session** must exist
for a host to **mirror/reflect an asset**. That session is the ceremony:

1. A host resolves the asset (TrustChain as decentralized DNS, via eBPF).
2. TrustChain establishes the certificate between the devices for that session.
3. NGauge dispatches which locality/mirrors serve which blocks and shards.
4. The host mirrors/reflects the asset — validating it independently, and thereby becoming a mirror
   of it itself.

Access *is* mirroring. To hold and validate an asset is to reflect it for others.

---

## 5. The Recursion — HyperMesh Defined in Itself

To make HyperMesh **fully Native**, we write the **full instruction set for the network** via our
**`SystemAsset` definitions**, and **register the mesh itself and DNS via the BlockMatrix itself.**

### ★ The SystemAssets become the genesis block of any given mesh network.

A network **defines itself at birth.** Its instruction set, its mesh registration, and its DNS all
instantiate as SystemAssets in the genesis block.

Three consequences, each of which resolves a previously-floating question:

- **"Validation = the rules of the network for a given asset"** now has a physical address. A
  network's rules **are** its genesis SystemAssets. There is no global requirements object — you
  read the rules from that network's genesis.
- **Scope/Tracking inheritance** resolves the same way. An asset inherits its
  Bounded|Unbounded × Tracked|Untracked posture from its network — that is, from that network's
  genesis. Nested sub-meshes are nested genesis definitions.
- **Joining a network means adopting its genesis** — its instruction set.

---

## 6. Standing Invariants

These govern everything above and are not negotiable:

- **PoS is authorization, never a magnitude.** No stake amounts, no coin logic. **Owner** = the
  distribution right. **Grantee** = access, and a grantee *is a mirror*. **Proxy** = transport only,
  content-opaque.
- **PoWork is the hash-validation** — 1:1 content correlation, not a difficulty contest.
- **Consensus is a result, not a factor.** It emerges from independent per-mirror validation. No
  voting, no quorum, no leader election.
- **Verification is mandatory.** STOQ requires eBPF; there is no degraded fallback, and dropping
  unverifiable traffic is correct behavior.
- **Load-balancing is emergent**, torrent-like, driven by engagement. Never an imposed quota. A
  node's resources are user-managed and volunteered electively.
- **Addresses are durable-by-derivation** — recomputed, never leased.

---

## 7. Where the Current Code Diverges

Recorded so the gap is explicit. Verify before acting on any of it.

- **DNS lives in `blockmatrix/src/dns/`.** Under this architecture it belongs to **TrustChain**,
  which is the decentralized DNS.
- **BlockMatrix is a monolith** — it currently carries IPC, dashboard, gateway, transfer, messaging,
  sharing, and DNS. It should be the *distributed storage substrate*. The rest is misplaced and
  needs re-homing.
- **NGauge carries analytics beyond dispatch.** Dispatch is the entire mandate.
- **Proof-of-State types are scattered across four crates.** They consolidate into **`lib`**.
- **TrustChain is due a facelift** to take up its role as decentralized DNS + session CA.
