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
| **NGauge** (`ngauge`) | **Resource dispatching — and that is all it has to do.** Two faces: an **interface for Catalog**, and a **scheduler for BlockMatrix**. Shard-distributes blocks. |
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

## 7. Security Model

> **You cannot keep something secret unless you are the only one who knows it.**

Everything else follows from taking that literally.

A secret shared is not a secret — it is a liability with a countdown. So HyperMesh holds exactly one
class of secret: **your private key, which never leaves you.** It does not cross a process boundary,
a C ABI, a wire, or a disk in cleartext. Nothing else in the system is treated as confidential-
because-hidden, because nothing else can be.

The inversion that makes this work: **secrecy is singular, but proof is plural.** You are the only
one who knows your key — and *everyone* mirrors the evidence produced with it. Trust is not
established by sharing secrets; it is established by publishing artifacts that are unforgeable
without a secret nobody else holds.

So verification arrives along two independent axes, and they corroborate each other:

- **Vertically — through the individual's own chain.** Your genesis signs itself, every subsequent
  entry chains to its predecessor, and the whole lineage is re-derivable from artifacts you
  published. One identity, one unbroken history.
- **Horizontally — through the mesh, because everyone is mirroring it.** A grantee *is* a mirror.
  Every mirror independently validates what it holds, and the distribution graph is itself the
  audit trail. To rewrite history you would have to rewrite it everywhere it is reflected,
  simultaneously, without any mirror noticing — while lacking the key that made it valid in the
  first place.

Neither axis requires you to reveal anything. The key stays singular; the proof goes everywhere.

### 7.1 Genesis is self-signing — the root of trust

**There is no authority above genesis.** A node's genesis block is not signed by a CA, a foundation
key, or a peer. It signs itself, and that is the entire root of trust:

- The **identity that signs genesis is declared inside genesis** (the Identity SystemAsset carries
  the FALCON-1024 public key).
- The **genesis hash is derived from that content**, including the identity — so the signer and the
  signed are bound in one object.
- The **chain identity IS the genesis hash**. Alter anything and you have a different chain, not a
  tampered one.

This is only meaningful because genesis is **deterministic** — a pure function of
(device assessment, matrix coordinate, genesis epoch). A peer re-derives it from inputs the block
itself carries and checks the hash. Non-deterministic genesis cannot be adopted, only asserted.

**Self-signing is not self-*certifying*.** Anyone can mint a genesis. What they cannot do is mint
one for *someone else's device fingerprint* without that device's private key, or alter one without
changing the identity of the chain it roots. Trust does not come from genesis being blessed; it
comes from genesis being **unforgeable, reproducible, and self-consistent**, and from every
subsequent claim chaining back to it.

### 7.2 Keyed hashing — proving possession without disclosure

A hash **salted with a private key** (BLAKE3 keyed mode) yields a value only the key-holder can
produce. Paired with a FALCON signature over that value, it gives possession-proof without
disclosure — the verifier confirms *the holder computed this* without ever seeing the key.

This is already the pattern behind DNS invitation tokens (BLAKE3-HMAC keyed by owner proof). The
generalization: **device continuity, epoch binding, and identity assertions can all be keyed rather
than revealed.** The private key never crosses a boundary — consistent with the FFI rule that
secret keys never cross the C ABI.

### 7.3 The store is never trusted; the content proves itself

Every asset is **content-addressed** (BLAKE3 = identity), **encrypted**, and **PoS-validated**.
Therefore the medium holding it is outside the trust boundary *by construction*.

The consequence is liberating: **any dumb carrier is safe.** A kernel eBPF map, a shared cache — or
an external store like etcd — can hold HyperMesh data without joining the TCB, because it cannot
read the content, cannot forge it, and cannot alter it undetected. Such a store is not a database we
trust; it is a **carrier we don't have to**. This is what makes distributed kernel-visible state
possible without weakening zero-trust, and it is why "no foreign databases" is about *trusted*
state, not about bytes at rest.

### 7.4 Verification is mandatory — black-holing is correct

STOQ **requires** eBPF. There is no degraded userspace fallback. A node that cannot verify does not
participate, and unverifiable traffic is **dropped**. In a zero-trust system an unverified path *is*
the vulnerability, so the absence of a fallback is a feature.

(The Gateway is the deliberate exception — it exists for IPv4/HTTP backwards-compatibility, and its
non-HyperMesh traffic must pass through. HyperMesh's own IPv6/QUIC plane is verified-or-dropped.)

### 7.5 Provenance is tamper-evident, not merely recorded

Each asset entry's identity **cryptographically depends on its predecessor's**: the lineage pointer
lives inside the `StateProof`, so `proof_hash = BLAKE3(state_proof)` chains entries, and
`Block::calculate_hash` folds that per entry. Rewriting an asset's history changes every downstream
hash.

The property this buys, stated precisely: **a forgery that passes every signature check is still
rejected.** An attacker can produce an entry that is validly FALCON-signed by a real identity, with
a correctly re-derived `proof_hash` and intact content binding — and the lineage gate rejects it
anyway, because the predecessor it names is not the head we recorded. Signatures prove *who*;
lineage proves *what came before*. Both are required.

### 7.6 Authorization is identity, never magnitude

**PoStake is authorization** — who owns, and to whom access is granted — verified cryptographically.
It is never a quantity, never a threshold, never a balance. There is no amount that buys authority.
Mirrors are grantees; grantees are identities; identities are `BLAKE3(FALCON pubkey)`.

Enforced mechanically: `scripts/check-no-pos-magnitude.sh` scans the **whole worktree** — including
orphaned and undeclared files, which is where three consecutive cleanup passes hid regressions — and
blocks on push.

### 7.7 Consensus is a result, not a factor

Each mirror validates independently. Agreement is the *emergent output* of independent validation,
never an input to it. There is no vote, no quorum, no leader election, and therefore no quorum to
capture or leader to compromise.

---

## 8. Where the Current Code Diverges

Recorded so the gap is explicit. Verify before acting on any of it.

- **DNS lives in `blockmatrix/src/dns/`.** Under this architecture it belongs to **TrustChain**,
  which is the decentralized DNS.
- **BlockMatrix is a monolith** — it currently carries IPC, dashboard, gateway, transfer, messaging,
  sharing, and DNS. It should be the *distributed storage substrate*. The rest is misplaced and
  needs re-homing.
- **NGauge carries analytics beyond dispatch.** Dispatch is the entire mandate.
- **Proof-of-State types are scattered across four crates.** They consolidate into **`lib`**.
- **TrustChain is due a facelift** to take up its role as decentralized DNS + session CA.
