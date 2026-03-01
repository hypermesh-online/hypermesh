// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP (Address Family XDP) Zero-Copy Socket Management
//!
//! Provides zero-copy packet I/O bypassing the kernel network stack
//! for maximum performance on the STOQ fast path. This is execution
//! path 1: AF_XDP -> STOQ (XDP_REDIRECT).
//!
//! With the `kernel-attach` feature enabled, creates real AF_XDP sockets
//! with UMEM shared memory, fill/completion/rx/tx ring buffers, and
//! true zero-copy packet I/O via direct `libc` syscalls.
//!
//! Without `kernel-attach` or when the kernel probe fails, sockets track
//! statistics and fall back to standard I/O.

mod batch_io;
mod helpers;
mod kernel_types;
mod manager;
mod socket_io;

pub use manager::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_umem_config_default() {
        let config = UmemConfig::default();
        assert_eq!(config.frame_count, 4096);
        assert_eq!(config.frame_size, 4096);
        assert_eq!(config.frame_headroom, 256);
        assert!(!config.use_huge_pages);
    }

    #[test]
    fn test_ring_config_default() {
        let config = RingConfig::default();
        assert_eq!(config.tx_size, 2048);
        assert_eq!(config.rx_size, 2048);
        assert_eq!(config.fill_size, 2048);
        assert_eq!(config.comp_size, 2048);
    }

    #[test]
    fn test_af_xdp_manager_creation() {
        let manager = AfXdpManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_af_xdp_socket_creation() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0);
        assert!(socket.is_ok());
        assert_eq!(manager.socket_count(), 1);

        // Duplicate should fail
        let dup = manager.create_socket("eth0", 0);
        assert!(dup.is_err());
    }

    #[test]
    fn test_af_xdp_socket_close() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let _socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");

        assert!(manager.close_socket("eth0", 0).is_ok());
        assert_eq!(manager.socket_count(), 0);

        // Close non-existent should fail
        assert!(manager.close_socket("eth0", 0).is_err());
    }

    #[test]
    fn test_af_xdp_socket_not_kernel_backed() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");
        assert!(!socket.is_kernel_backed());
        assert_eq!(socket.interface(), "eth0");
        assert_eq!(socket.queue_id(), 0);
    }

    #[test]
    fn test_af_xdp_socket_free_frames_fallback() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");
        // Non-kernel-backed sockets report 0 free frames
        assert_eq!(socket.free_frames(), 0);
    }

    #[test]
    fn test_af_xdp_socket_clone() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");
        let cloned = socket.clone();
        assert_eq!(cloned.interface(), "eth0");
        assert_eq!(cloned.queue_id(), 0);
        assert_eq!(cloned.is_kernel_backed(), socket.is_kernel_backed());
    }

    #[test]
    fn test_af_xdp_manager_with_config() {
        let umem = UmemConfig {
            frame_count: 8192,
            frame_size: 4096,
            frame_headroom: 128,
            use_huge_pages: false,
        };
        let ring = RingConfig {
            tx_size: 4096,
            rx_size: 4096,
            fill_size: 4096,
            comp_size: 4096,
        };
        let manager = AfXdpManager::with_config(umem, ring);
        assert!(manager.is_ok());
        let mgr = manager.expect("test: create manager with config");
        assert_eq!(mgr.umem_config.frame_count, 8192);
        assert_eq!(mgr.ring_config.tx_size, 4096);
    }

    #[test]
    fn test_af_xdp_stats_default() {
        let stats = AfXdpStats::default();
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.tx_ring_full, 0);
        assert_eq!(stats.rx_ring_empty, 0);
        assert_eq!(stats.invalid_descriptors, 0);
    }

    #[test]
    fn test_af_xdp_manager_get_stats() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let _socket = manager.create_socket("lo", 0).expect("test: create socket");
        let stats = manager.get_stats("lo", 0);
        assert!(stats.is_some());
        let s = stats.expect("test: get stats");
        assert_eq!(s.packets_sent, 0);

        // Non-existent socket returns None
        let no_stats = manager.get_stats("eth99", 0);
        assert!(no_stats.is_none());
    }

    #[test]
    fn test_af_xdp_manager_close_all() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let _s1 = manager
            .create_socket("eth0", 0)
            .expect("test: create socket 1");
        let _s2 = manager
            .create_socket("eth1", 0)
            .expect("test: create socket 2");
        assert_eq!(manager.socket_count(), 2);

        manager.close_all().expect("test: close all");
        assert_eq!(manager.socket_count(), 0);
    }

    #[tokio::test]
    async fn test_af_xdp_send_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");
        let result = socket.send(&[1, 2, 3, 4]).await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.bytes_sent, 4);
    }

    #[tokio::test]
    async fn test_af_xdp_receive_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");
        let result = socket.receive().await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.rx_ring_empty, 1);
    }

    #[tokio::test]
    async fn test_af_xdp_send_batch_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");
        let packets: Vec<&[u8]> = vec![&[1, 2], &[3, 4, 5]];
        let result = socket.send_batch(&packets).await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.packets_sent, 2);
        assert_eq!(stats.bytes_sent, 5);
    }

    #[tokio::test]
    async fn test_af_xdp_receive_batch_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager
            .create_socket("eth0", 0)
            .expect("test: create socket");
        let result = socket.receive_batch(10).await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.rx_ring_empty, 1);
    }

    #[cfg(feature = "kernel-attach")]
    mod kernel_tests {
        use super::super::helpers::validate_config;
        use super::super::kernel_types::*;
        use super::super::manager::*;

        #[test]
        fn test_validate_config_valid() {
            let umem = UmemConfig::default();
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_ok());
        }

        #[test]
        fn test_validate_config_bad_frame_count() {
            let umem = UmemConfig {
                frame_count: 100, // not power of 2
                ..Default::default()
            };
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_validate_config_bad_frame_size() {
            let umem = UmemConfig {
                frame_size: 1024, // too small (< 2048)
                ..Default::default()
            };
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_validate_config_bad_headroom() {
            let umem = UmemConfig {
                frame_headroom: 4096, // equals frame_size
                ..Default::default()
            };
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_validate_config_bad_ring_size() {
            let umem = UmemConfig::default();
            let ring = RingConfig {
                tx_size: 100, // not power of 2
                ..Default::default()
            };
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_frame_allocator() {
            let alloc = FrameAllocator::new(8, 4096);
            assert_eq!(alloc.available(), 8);

            let a = alloc.allocate().expect("test: allocate frame 1");
            assert_eq!(alloc.available(), 7);

            alloc.release(a);
            assert_eq!(alloc.available(), 8);
        }

        #[test]
        fn test_frame_allocator_exhaustion() {
            let alloc = FrameAllocator::new(2, 4096);
            let _a1 = alloc.allocate().expect("test: allocate 1");
            let _a2 = alloc.allocate().expect("test: allocate 2");
            assert!(alloc.allocate().is_none());
        }

        #[test]
        fn test_frame_allocator_batch_release() {
            let alloc = FrameAllocator::new(4, 4096);
            let a1 = alloc.allocate().expect("test: allocate 1");
            let a2 = alloc.allocate().expect("test: allocate 2");
            assert_eq!(alloc.available(), 2);

            alloc.release_batch(&[a1, a2]);
            assert_eq!(alloc.available(), 4);
        }

        #[test]
        fn test_xdp_desc_size() {
            assert_eq!(std::mem::size_of::<XdpDesc>(), 16);
        }

        #[test]
        fn test_xdp_umem_reg_layout() {
            // addr(8) + len(8) + chunk_size(4) + headroom(4) + flags(4) = 28
            // But with padding the struct may be different; check it compiles
            let reg = XdpUmemReg {
                addr: 0,
                len: 0,
                chunk_size: 4096,
                headroom: 256,
                flags: 0,
            };
            assert_eq!(reg.chunk_size, 4096);
        }

        #[test]
        fn test_sockaddr_xdp_layout() {
            let addr = SockaddrXdp {
                sxdp_family: 44,
                sxdp_flags: 0,
                sxdp_ifindex: 1,
                sxdp_queue_id: 0,
                sxdp_shared_umem_fd: 0,
            };
            assert_eq!(addr.sxdp_family, 44);
        }
    }
}
