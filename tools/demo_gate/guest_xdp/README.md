# Guest-only HyperMesh XDP kernel diagnostic

`./tools/demo_gate/guest_xdp/run.sh` compiles the **tracked**
`hypermesh-ebpf/programs/hypermesh_xdp.c` with its `build.rs` BPF flags and
boots a disposable Linux guest. The guest uses direct libbpf to load the exact
object, attach generic XDP to guest loopback, send **one ordinary IPv6 UDP
packet to port 9292** and **one to control port 9293**, read the program's
`connection_map`, detach, and confirm that the kernel reports no attachment.
This is not STOQ protocol traffic or the production Aya loader.

Requirements: Linux KVM (`/dev/kvm`), packaged kernel (default
`/boot/vmlinuz-linux`), clang BPF target, GCC, libbpf headers/runtime, QEMU,
`ldd`, `cpio`, gzip, timeout, and a writable SSD output root (default
`/usr/lib/hypermesh/verification`). Set `HYPERMESH_GUEST_KERNEL` or
`HYPERMESH_GUEST_OUT_ROOT` to override. No installation or download occurs.
The script refuses KVM failure rather than silently falling back to emulation.

Isolation: 512 MiB, one vCPU, 90-second direct QEMU timeout, `-nic none`, no
disks or host mounts. Guest root is a minimal initramfs with only this runner,
the project BPF object, and host-distro runtime libraries. All artifacts and
raw logs stay in a mode-0700 SSD directory printed as `evidence=...`; nothing
is written to the host kernel or tracked source tree by the run.

Pass requires a real kernel XDP program ID, matching attached ID, at least one
port-9292 connection-map entry with packet count, zero port-9293 entries, and
confirmed detachment (`post_id=0`). Exit is nonzero on setup, QEMU, guest,
traffic-attribution or detach failures. A pass establishes this narrow guest
kernel prerequisite only; it does **not** establish production STOQ/eBPF usage,
PoS/Catalog authorization, transfer-linked counters, two-node functionality,
or asset durability.
