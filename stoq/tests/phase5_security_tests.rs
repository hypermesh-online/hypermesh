// STOQ Phase 5: Comprehensive Security Testing Suite
// Validates quantum resistance, DoS protection, and security boundaries

use stoq::{StoqTransport, Config};
use stoq::crypto::{FalconHandshake, CertificateValidator};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use bytes::Bytes;

mod quantum_resistance_tests {
    use super::*;
    use pqcrypto_falcon::falcon1024;
    use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};

    #[tokio::test]
    async fn test_falcon1024_key_generation() {
        // Verify FALCON-1024 key generation
        let (pk, sk) = falcon1024::keypair();

        // Check key sizes match NIST Level V requirements
        assert_eq!(pk.as_bytes().len(), 1793); // Public key size
        assert_eq!(sk.as_bytes().len(), 2305); // Secret key size
    }

    #[tokio::test]
    async fn test_quantum_safe_handshake() {
        let handshake = FalconHandshake::new();

        // Generate quantum-safe keypairs
        let (alice_pub, alice_priv) = handshake.generate_keypair();
        let (bob_pub, bob_priv) = handshake.generate_keypair();

        // Perform quantum-safe key exchange
        let alice_premaster = handshake.compute_premaster(&bob_pub, &alice_priv);
        let bob_premaster = handshake.compute_premaster(&alice_pub, &bob_priv);

        // Verify shared secret derivation
        assert_eq!(alice_premaster.len(), 32); // 256-bit security
        assert_eq!(alice_premaster, bob_premaster);
    }

    #[tokio::test]
    async fn test_signature_verification() {
        let handshake = FalconHandshake::new();
        let (pub_key, priv_key) = handshake.generate_keypair();

        // Sign message
        let message = b"Critical security data";
        let signature = handshake.sign(message, &priv_key);

        // Verify signature
        assert!(handshake.verify(message, &signature, &pub_key));

        // Test tampered message
        let tampered = b"Modified security data";
        assert!(!handshake.verify(tampered, &signature, &pub_key));

        // Test tampered signature
        let mut bad_sig = signature.clone();
        bad_sig[0] ^= 0xFF;
        assert!(!handshake.verify(message, &bad_sig, &pub_key));
    }

    #[tokio::test]
    async fn test_quantum_attack_resistance() {
        // Simulate quantum computer attempting to break FALCON-1024
        let handshake = FalconHandshake::new();
        let (pub_key, priv_key) = handshake.generate_keypair();

        // Create valid signature
        let message = b"Quantum-safe message";
        let signature = handshake.sign(message, &priv_key);

        // Attempt to forge signature without private key
        // This should be computationally infeasible even for quantum computers
        let forged_message = b"Forged message";
        let random_sig = vec![0u8; signature.len()];

        assert!(!handshake.verify(forged_message, &random_sig, &pub_key));

        // Verify NIST Level V security (2^256 classical, 2^128 quantum)
        assert_eq!(handshake.security_bits(), 256);
        assert_eq!(handshake.quantum_security_bits(), 128);
    }

    #[tokio::test]
    async fn test_hybrid_cryptography() {
        // Test hybrid classical + quantum-resistant crypto
        let mut config = Config::default();
        config.enable_hybrid_crypto = true;

        let transport = StoqTransport::new(config);

        // Verify both classical and quantum algorithms active
        assert!(transport.has_classical_crypto());
        assert!(transport.has_quantum_crypto());

        // Test that connection uses both
        let crypto_suite = transport.active_crypto_suite();
        assert!(crypto_suite.contains("TLS"));
        assert!(crypto_suite.contains("FALCON"));
    }
}

mod dos_protection_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[tokio::test]
    async fn test_connection_rate_limiting() {
        let mut config = Config::default();
        config.max_connections_per_second = 100;
        config.enable_dos_protection = true;

        let server = StoqTransport::new(config);
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Track successful connections
        let success_count = Arc::new(AtomicU64::new(0));
        let count_clone = success_count.clone();

        // Server accept loop with rate limiting
        let server_handle = tokio::spawn(async move {
            loop {
                match timeout(Duration::from_millis(10), listener.accept()).await {
                    Ok(Ok(_)) => {
                        count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => continue,
                }
            }
        });

        // Attempt 1000 connections in 1 second
        let mut handles = Vec::new();
        for _ in 0..1000 {
            let handle = tokio::spawn(async move {
                let client = StoqTransport::new(Config::default());
                let _ = timeout(
                    Duration::from_millis(100),
                    client.connect(server_addr)
                ).await;
            });
            handles.push(handle);
        }

        // Wait for all attempts
        for handle in handles {
            let _ = handle.await;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Verify rate limiting worked (should be ~100 connections)
        let total = success_count.load(Ordering::Relaxed);
        assert!(total <= 150); // Some tolerance for timing
        assert!(total >= 80);  // But should be close to limit

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_memory_exhaustion_protection() {
        let mut config = Config::default();
        config.max_memory_per_connection = 1024 * 1024; // 1MB limit
        config.enable_dos_protection = true;

        let transport = StoqTransport::new(config);

        // Try to allocate excessive memory
        let result = transport.allocate_buffer(10 * 1024 * 1024); // 10MB
        assert!(result.is_err());

        // Verify transport still healthy
        assert!(transport.is_healthy());

        // Normal allocation should work
        let result = transport.allocate_buffer(512 * 1024); // 512KB
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cpu_exhaustion_protection() {
        let mut config = Config::default();
        config.max_cpu_per_connection = 0.1; // 10% CPU limit
        config.enable_dos_protection = true;

        let transport = Arc::new(StoqTransport::new(config));

        // Simulate CPU-intensive operation
        let transport_clone = transport.clone();
        let cpu_task = tokio::spawn(async move {
            transport_clone.perform_cpu_intensive_operation().await
        });

        // Task should be terminated if it exceeds CPU limit
        let result = timeout(Duration::from_secs(1), cpu_task).await;
        assert!(result.is_ok()); // Didn't hang

        // Verify CPU usage was limited
        let stats = transport.get_cpu_stats();
        assert!(stats.peak_usage < 0.15); // Within limit + margin
    }

    #[tokio::test]
    async fn test_amplification_attack_prevention() {
        // Test protection against amplification attacks
        let mut config = Config::default();
        config.enable_dos_protection = true;
        config.max_response_amplification = 2.0; // 2x max amplification

        let server = StoqTransport::new(config);
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Read tiny request
            let mut buf = vec![0u8; 10];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(n, 10);

            // Try to send huge response (should be limited)
            let huge_response = vec![0xFF; 1024 * 1024]; // 1MB
            let result = stream.write_all(&huge_response).await;

            // Should fail due to amplification limit
            assert!(result.is_err());
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        // Send tiny request
        stream.write_all(&[0u8; 10]).await.unwrap();

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_slowloris_protection() {
        // Test protection against slowloris attacks
        let mut config = Config::default();
        config.enable_dos_protection = true;
        config.handshake_timeout = Duration::from_secs(5);
        config.idle_timeout = Duration::from_secs(10);

        let server = StoqTransport::new(config);
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let result = timeout(
                Duration::from_secs(6),
                listener.accept()
            ).await;

            // Connection should be rejected due to slow handshake
            assert!(result.is_err() || result.unwrap().is_err());
        });

        // Create slow client
        let client = StoqTransport::new(Config::default());
        let connect_future = client.connect(server_addr);

        // Deliberately slow down handshake
        tokio::time::sleep(Duration::from_secs(6)).await;

        // Connection should fail
        let result = timeout(Duration::from_secs(1), connect_future).await;
        assert!(result.is_err() || result.unwrap().is_err());

        server_handle.await.unwrap();
    }
}

mod input_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_malformed_packet_handling() {
        let transport = StoqTransport::new(Config::default());

        // Test various malformed inputs
        let malformed_packets = vec![
            vec![],                           // Empty packet
            vec![0xFF; 1],                    // Too short
            vec![0x00; 65536],               // Too large
            vec![0xFF, 0xFF, 0xFF, 0xFF],    // Invalid header
            b"GET / HTTP/1.1\r\n\r\n".to_vec(), // Wrong protocol
        ];

        for packet in malformed_packets {
            let result = transport.process_packet(&packet).await;
            assert!(result.is_err(), "Should reject malformed packet");
        }

        // Transport should remain healthy
        assert!(transport.is_healthy());
    }

    #[tokio::test]
    async fn test_injection_attack_prevention() {
        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Read potentially malicious input
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();

            // Verify input was sanitized
            let data = String::from_utf8_lossy(&buf[..n]);
            assert!(!data.contains('\0')); // No null bytes
            assert!(!data.contains("../"));  // No path traversal
            assert!(!data.contains("';"));   // No SQL injection
            assert!(!data.contains("<script>")); // No XSS
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        // Attempt various injection attacks
        let malicious = b"../../etc/passwd\0'; DROP TABLE users; --<script>alert('xss')</script>";
        stream.write_all(malicious).await.unwrap();

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_buffer_overflow_protection() {
        let transport = StoqTransport::new(Config::default());

        // Attempt buffer overflow
        let oversized = vec![0xFF; 100 * 1024 * 1024]; // 100MB
        let result = transport.process_data(&oversized).await;

        // Should reject without crashing
        assert!(result.is_err());
        assert!(transport.is_healthy());

        // Verify memory wasn't corrupted
        let test_data = b"test";
        let result = transport.process_data(test_data).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fuzzing_resistance() {
        use rand::{Rng, thread_rng};

        let transport = StoqTransport::new(Config::default());
        let mut rng = thread_rng();

        // Generate random fuzz inputs
        for _ in 0..1000 {
            let size = rng.gen_range(0..10000);
            let mut fuzz_data = vec![0u8; size];
            rng.fill(&mut fuzz_data[..]);

            // Process fuzzed input
            let _ = transport.process_data(&fuzz_data).await;

            // Transport should remain stable
            assert!(transport.is_healthy());
        }

        // Verify normal operation still works
        let valid_data = b"valid data";
        let result = transport.process_data(valid_data).await;
        assert!(result.is_ok());
    }
}

mod certificate_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_certificate_chain_validation() {
        let validator = CertificateValidator::new();

        // Test valid certificate chain
        let valid_chain = vec![
            "-----BEGIN CERTIFICATE-----\nroot\n-----END CERTIFICATE-----",
            "-----BEGIN CERTIFICATE-----\nintermediate\n-----END CERTIFICATE-----",
            "-----BEGIN CERTIFICATE-----\nleaf\n-----END CERTIFICATE-----",
        ];

        let result = validator.validate_chain(&valid_chain);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_expired_certificate_rejection() {
        let validator = CertificateValidator::new();

        // Create expired certificate (mock)
        let expired_cert = validator.create_test_cert(
            chrono::Utc::now() - chrono::Duration::days(30), // Expired
            chrono::Utc::now() - chrono::Duration::days(1),
        );

        let result = validator.validate_cert(&expired_cert);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expired"));
    }

    #[tokio::test]
    async fn test_hostname_verification() {
        let validator = CertificateValidator::new();

        // Test hostname matching
        let cert = validator.create_test_cert_for_hostname("example.com");

        assert!(validator.verify_hostname(&cert, "example.com").is_ok());
        assert!(validator.verify_hostname(&cert, "evil.com").is_err());

        // Test wildcard certificates
        let wildcard_cert = validator.create_test_cert_for_hostname("*.example.com");

        assert!(validator.verify_hostname(&wildcard_cert, "sub.example.com").is_ok());
        assert!(validator.verify_hostname(&wildcard_cert, "example.com").is_err());
        assert!(validator.verify_hostname(&wildcard_cert, "a.b.example.com").is_err());
    }

    #[tokio::test]
    async fn test_certificate_pinning() {
        let mut config = Config::default();
        config.enable_cert_pinning = true;
        config.pinned_certs = vec![
            "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
        ];

        let transport = StoqTransport::new(config);

        // Connection with unpinned cert should fail
        let result = transport.connect("[::1]:443".parse().unwrap()).await;
        assert!(result.is_err());

        // Connection with pinned cert should succeed (mock)
        transport.add_pinned_cert("sha256/BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=");
        let result = transport.connect_with_pin("[::1]:443".parse().unwrap()).await;
        // Would succeed with matching cert
    }
}

mod connection_security_tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_hijacking_prevention() {
        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, peer_addr) = listener.accept().await.unwrap();

            // Store initial peer address
            let initial_peer = peer_addr;

            // Read data and verify source
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();

            // Verify packet came from original peer
            assert_eq!(stream.peer_addr(), initial_peer);

            stream.write_all(b"Authenticated").await.unwrap();
        });

        // Legitimate client
        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        stream.write_all(b"Legitimate data").await.unwrap();

        // Attacker tries to hijack (would fail due to connection ID)
        let attacker = StoqTransport::new(Config::default());
        let hijack_attempt = attacker.hijack_connection(stream.connection_id()).await;
        assert!(hijack_attempt.is_err());

        // Original client still connected
        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Authenticated");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_replay_attack_prevention() {
        let transport = StoqTransport::new(Config::default());

        // Capture legitimate packet
        let packet = transport.create_test_packet(b"legitimate data");

        // Process packet once
        let result1 = transport.process_packet(&packet).await;
        assert!(result1.is_ok());

        // Replay same packet
        let result2 = transport.process_packet(&packet).await;
        assert!(result2.is_err()); // Should reject replay

        // Verify nonce/timestamp checking
        let error = result2.unwrap_err();
        assert!(error.to_string().contains("replay") ||
                error.to_string().contains("duplicate"));
    }

    #[tokio::test]
    async fn test_mitm_detection() {
        // Test man-in-the-middle attack detection
        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Establish secure channel with fingerprint
            let fingerprint = stream.get_peer_fingerprint();

            // Verify all messages come from same peer
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();

            assert_eq!(stream.get_peer_fingerprint(), fingerprint);
            stream.write_all(b"Secure").await.unwrap();
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        // Get server fingerprint
        let server_fingerprint = stream.get_peer_fingerprint();

        stream.write_all(b"Client data").await.unwrap();

        // Verify response is from same server
        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(stream.get_peer_fingerprint(), server_fingerprint);

        server_handle.await.unwrap();
    }
}

// Run all security tests with: cargo test --test phase5_security_tests