# Independent kernel-attachment prerequisite, never a demo verifier

Run `python3 tools/demo_gate/kernel_xdp_preflight.py --interface <local-interface>`.
Only fixed read-only bpftool queries run: no loading, attaching, packet capture,
sudo, provisioning, or configuration changes. Exit 0 means ONLY stable kernel attachment
was observed; exit 2 means unavailable/unobserved/error. `demo_verified` is always
false, including exit 0. There is no evidence-JSON input,
counter override, permissive mode or fallback pass. Raw query results accompany
the observation; missing tools, denied queries and attachment races fail closed.

An observed ID/tag/name could be unrelated to STOQ. This answers ONLY whether
kernel inventory exposed an XDP attachment on this interface at observation time.
It does not establish approved source binding, traffic through that program, or
authorization. Actual transfer-linked kernel counters and idle/negative controls
are still required, not generic activity or application-reported counters.

## Concrete production gaps at fde0a9a0

- `stoq/src/transport/manager/kernel_acceleration.rs::initialize` detects capability;
  `attach_and_create_socket` warns after attachment failure and continues.
- `stoq/src/transport/operations.rs::send` falls back to Quinn on AF_XDP failure.
  Successful sending or an allocated object therefore cannot prove XDP use.
- `blockmatrix/src/os_integration/linux/ebpf.rs::read_ebpf_metrics_impl` explicitly
  returns SIMULATION ONLY constants, including packets_processed=12345.
- `hypermesh-ebpf/programs/hypermesh_xdp.c::hypermesh_xdp_filter` parses cleartext
  headers; `stoq/src/transport/pos_extension_validator.rs::validate` documents
  structural checks and deferred full crypto. Neither proves authentic fresh,
  action/asset/network-scoped authorization on its own.

The trial also needs two independent production identities, a canonical Catalog
asset and authorized BlockMatrix transfer/reconstruction with identical bytes.
This prerequisite manufactures none of those facts and accepts no boolean claims
as their verification. No public or privileged kernel trial is authorized here.

Run refusal unit tests with
`python3 -m unittest discover -s tests/unit/demo_gate -v`.
Unit query fixtures are NOT kernel, transport, proof or two-node evidence.

Interface identity is bound to the independently resolved local ifindex before
and after the query. The full (mode, program-ID) inventory must remain identical;
mode changes or interface replacement are refusals. Upstream multi_attachments
records are explicitly UNSUPPORTED and refused, never silently omitted. The probe
uses only fixed system bpftool paths; resolved executable and parents must be
root-owned and not group/other writable. Arbitrary PATH executables are ignored.
