# Watch a STOQ message reach the Linux kernel

[Download the MP4](./HyperMesh-STOQ-demo.mp4?raw=true) · [Transcript](./HyperMesh-STOQ-transcript.txt) · [How to reproduce](../../../tools/demo_gate/stoq_guest/README.md)

This 45-second silent clip replays captured output, not a live screen recording. Inside one Linux guest, a STOQ client sent 41 bytes and checked the echo. Watch the HyperMesh XDP packet counter: it rose from 21 to 23 on the target port, then stayed at 25 during a second echo on a different port. These counts are packets, not bytes, and may include QUIC control frames.

That demonstrates one real transport path. Next: two independently installed nodes transferring a Catalog-backed BlockMatrix asset authorized by Proof of State. This is not a public alpha.

**Separate storage result:** [BlockMatrix](../../../blockmatrix/examples/pipeline_demo.rs) reconstructed 36,000 bytes locally from its four-data/two-parity shard set. The bytes matched the original. This was not distributed retrieval or part of the filmed exchange.
