// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate persistence to filesystem for reuse across restarts.

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

use super::types::StoqNodeCertificate;

/// Determine the filesystem cache directory for certificate persistence.
///
/// Falls back to `$HOME/.stoq/certs/<node_id>`, then `/tmp/stoq-certs/<node_id>`.
/// Returns `None` if no directory could be created.
pub fn resolve_cache_dir(node_id: &str) -> Option<std::path::PathBuf> {
    let base = std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(".stoq").join("certs"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/stoq-certs"));
    let dir = base.join(node_id);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Try to load a cached certificate from disk.
///
/// Returns `Some(cert)` if a valid (non-expired, at least 1h remaining) certificate
/// was loaded from the given cache directory.
pub fn try_load_cached_certificate(
    cache_dir: &std::path::Path,
    calculate_fingerprint: impl Fn(&[u8]) -> [u8; 32],
) -> Option<StoqNodeCertificate> {
    let cert_path = cache_dir.join("cert.der");
    let key_path = cache_dir.join("key.der");
    let meta_path = cache_dir.join("meta.json");

    let cert_bytes = std::fs::read(&cert_path).ok()?;
    let key_bytes = std::fs::read(&key_path).ok()?;
    let meta_bytes = std::fs::read(&meta_path).ok()?;

    #[derive(serde::Deserialize)]
    struct CertMeta {
        node_id: String,
        expires_at_secs: u64,
        issued_at_secs: u64,
    }

    let meta: CertMeta = serde_json::from_slice(&meta_bytes).ok()?;

    let expires_at = std::time::UNIX_EPOCH + Duration::from_secs(meta.expires_at_secs);
    let issued_at = std::time::UNIX_EPOCH + Duration::from_secs(meta.issued_at_secs);

    // Only use cached cert if it still has at least 1 hour of validity
    match expires_at.duration_since(SystemTime::now()) {
        Ok(remaining) if remaining >= Duration::from_secs(3600) => {}
        Ok(_) => {
            debug!("Cached certificate expires soon, will regenerate");
            return None;
        }
        Err(_) => {
            debug!("Cached certificate already expired");
            return None;
        }
    }

    let cert_der = CertificateDer::from(cert_bytes.clone());
    let private_key = PrivateKeyDer::try_from(key_bytes).ok()?;
    let fingerprint = calculate_fingerprint(&cert_bytes);

    let stoq_cert = StoqNodeCertificate {
        node_id: meta.node_id,
        certificate: cert_der,
        private_key,
        issued_at,
        expires_at,
        fingerprint_sha256: fingerprint,
        metadata: None,
    };

    // Validate cert-key consistency before returning. A mismatch
    // (e.g. one file regenerated but the other stale) causes rustls
    // KeyMismatch errors downstream. Auto-recover by deleting stale files.
    // Use an explicit CryptoProvider so this works even before the
    // process-level default is installed.
    let validation_result = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| e.to_string())
    .and_then(|b| {
        b.with_no_client_auth()
            .with_single_cert(
                vec![stoq_cert.certificate.clone()],
                stoq_cert.private_key.clone_key(),
            )
            .map_err(|e| e.to_string())
    });
    if let Err(e) = validation_result {
        warn!(
            "Stale certificates detected ({}), removing cached files in {}",
            e,
            cache_dir.display()
        );
        let _ = std::fs::remove_file(&cert_path);
        let _ = std::fs::remove_file(&key_path);
        let _ = std::fs::remove_file(&meta_path);
        return None;
    }

    info!("Loaded cached certificate from {}", cert_path.display());
    Some(stoq_cert)
}

/// Persist a certificate to disk for reuse across restarts.
pub fn persist_certificate(cache_dir: &std::path::Path, cert: &StoqNodeCertificate) {
    let cert_path = cache_dir.join("cert.der");
    let key_path = cache_dir.join("key.der");
    let meta_path = cache_dir.join("meta.json");

    let expires_secs = cert
        .expires_at
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let issued_secs = cert
        .issued_at
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let meta = serde_json::json!({
        "node_id": cert.node_id,
        "expires_at_secs": expires_secs,
        "issued_at_secs": issued_secs,
    });

    if let Err(e) = std::fs::write(&cert_path, cert.certificate.as_ref()) {
        debug!("Failed to persist certificate: {e}");
        return;
    }

    // Persist private key DER bytes
    let key_bytes: &[u8] = match &cert.private_key {
        PrivateKeyDer::Pkcs1(k) => k.secret_pkcs1_der(),
        PrivateKeyDer::Pkcs8(k) => k.secret_pkcs8_der(),
        PrivateKeyDer::Sec1(k) => k.secret_sec1_der(),
        _ => {
            debug!("Unsupported private key format for persistence");
            return;
        }
    };
    if let Err(e) = std::fs::write(&key_path, key_bytes) {
        debug!("Failed to persist private key: {e}");
        return;
    }

    if let Ok(json) = serde_json::to_string_pretty(&meta) {
        let _ = std::fs::write(&meta_path, json);
    }

    debug!("Persisted certificate to {}", cert_path.display());
}
