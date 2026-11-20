#!/usr/bin/env bash
# Real guest-kernel prerequisite diagnostic; never loads BPF on the host.
set -euo pipefail
umask 077

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
source_root=$(cd -- "$script_dir/../../.." && pwd)
source_c="$source_root/hypermesh-ebpf/programs/hypermesh_xdp.c"
out_root=${HYPERMESH_GUEST_OUT_ROOT:-/usr/lib/hypermesh/verification}
kernel=${HYPERMESH_GUEST_KERNEL:-/boot/vmlinuz-linux}

for tool in clang gcc ldd cpio gzip qemu-system-x86_64 timeout awk sha256sum \
    grep find cp mktemp date; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 2; }
done
test -f "$source_c" && test -f "$kernel" || { echo 'missing tracked C or kernel' >&2; exit 2; }
test -c /dev/kvm && test -r /dev/kvm && test -w /dev/kvm || {
    echo 'KVM unavailable; refusing emulation fallback' >&2; exit 2;
}
test -d "$out_root" && test -w "$out_root" || {
    echo "unwritable SSD output root: $out_root" >&2; exit 2;
}

out=$(mktemp -d "$out_root/guest_xdp-XXXXXXXX")
rootfs="$out/rootfs"
mkdir -p "$rootfs/proc" "$rootfs/sys"
printf 'source=%s\nkernel=%s\nstarted=%s\n' \
    "$source_c" "$kernel" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$out/manifest.txt"
printf 'evidence=%s\n' "$out"

includes=(-I/usr/include -I/usr/include/bpf)
if test -d /usr/include/x86_64-linux-gnu; then
    includes+=(-I/usr/include/x86_64-linux-gnu)
fi
printf '%s\n' 'compile=clang -O2 -g -target bpf -c tracked hypermesh_xdp.c -D__TARGET_ARCH_x86' >> "$out/manifest.txt"
clang -O2 -g -target bpf -c "$source_c" -o "$out/hypermesh_xdp.o" \
    "${includes[@]}" -D__TARGET_ARCH_x86 > "$out/clang.stdout" 2> "$out/clang.stderr"
gcc -O2 -Wall -Wextra "$script_dir/guest_init.c" -lbpf -o "$out/init" \
    > "$out/gcc.stdout" 2> "$out/gcc.stderr"

ldd "$out/init" > "$out/ldd.txt"
if grep -q 'not found' "$out/ldd.txt"; then
    echo 'missing guest runtime library' >&2; exit 2
fi
cp "$out/init" "$rootfs/init"
cp "$out/hypermesh_xdp.o" "$rootfs/hypermesh_xdp.o"
while IFS= read -r lib; do
    test -f "$lib" || { echo "missing runtime library: $lib" >&2; exit 2; }
    cp -L --parents "$lib" "$rootfs"
done < <(awk '$2=="=>" && $3 ~ "^/" {print $3} $1 ~ "^/" {print $1}' "$out/ldd.txt")
(cd "$rootfs" && find . -print0 | cpio --null -o -H newc 2> "$out/cpio.stderr") \
    | gzip -1 > "$out/initramfs.cpio.gz"
sha256sum "$source_c" "$script_dir/guest_init.c" "$out/hypermesh_xdp.o" \
    "$out/init" "$out/initramfs.cpio.gz" "$kernel" > "$out/hashes.txt"

printf 'qemu_start=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$out/manifest.txt"
printf '%s\n' 'qemu=KVM q35, 512MiB, 1vCPU, -nic none, no disks/mounts, 90s timeout' >> "$out/manifest.txt"
set +e
timeout -s TERM -k 3s 90s qemu-system-x86_64 \
    -machine q35,accel=kvm -cpu host -m 512M -smp 1 -nic none \
    -kernel "$kernel" -initrd "$out/initramfs.cpio.gz" \
    -append 'console=ttyS0 rdinit=/init panic=1 quiet' \
    -no-reboot -display none -serial stdio -monitor none \
    > "$out/qemu.stdout" 2> "$out/qemu.stderr"
qemu_exit=$?
set -e
printf 'qemu_exit=%s\nqemu_end=%s\n' \
    "$qemu_exit" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$out/manifest.txt"
sha256sum "$out/qemu.stdout" "$out/qemu.stderr" >> "$out/hashes.txt"
cat "$out/qemu.stdout"
if test "$qemu_exit" -ne 0; then
    echo "QEMU failed: exit $qemu_exit; see $out" >&2; exit "$qemu_exit"
fi
if ! grep -Fq 'GUEST: result=PASS code=0' "$out/qemu.stdout"; then
    echo "guest diagnostic failed; see $out" >&2; exit 1
fi
if ! grep -Fq 'GUEST: detach=0 post_query=0 post_id=0' "$out/qemu.stdout"; then
    echo "guest detach not confirmed; see $out" >&2; exit 1
fi
printf 'guest_result=PASS\n' >> "$out/manifest.txt"
