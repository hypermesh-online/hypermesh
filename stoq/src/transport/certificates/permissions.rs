// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Scope-based permission enforcement for peer certificates.

use anyhow::Result;
use tracing::debug;

use super::types::CertificateMode;

/// Check if a peer certificate is authorized for the given operation.
///
/// Inspects the X.509 Extended Key Usage (EKU) extensions:
/// - ServerAuth: peer may serve data (shard_send, gossip)
/// - ClientAuth: peer may request data (shard_fetch)
///
/// In LocalhostTesting mode all operations are permitted.
pub async fn check_peer_permission(
    mode: &CertificateMode,
    peer_cert_der: &[u8],
    operation: &str,
) -> Result<bool> {
    if *mode == CertificateMode::LocalhostTesting {
        return Ok(true);
    }

    match x509_parser::parse_x509_certificate(peer_cert_der) {
        Ok((_, parsed)) => {
            use x509_parser::extensions::ParsedExtension;

            // Extract EKU from extensions
            let mut has_server_auth = false;
            let mut has_client_auth = false;

            for ext in parsed.extensions() {
                if let ParsedExtension::ExtendedKeyUsage(eku) = ext.parsed_extension() {
                    has_server_auth = eku.any || eku.server_auth;
                    has_client_auth = eku.any || eku.client_auth;
                }
            }

            // If no EKU extension is present, allow all (backward compat)
            if !has_server_auth && !has_client_auth {
                return Ok(true);
            }

            match operation {
                "shard_fetch" => Ok(has_client_auth),
                "shard_send" | "gossip" => Ok(has_server_auth),
                _ => Ok(has_server_auth || has_client_auth),
            }
        }
        Err(_) => {
            debug!("Cannot parse peer certificate for permission check");
            Ok(false)
        }
    }
}
