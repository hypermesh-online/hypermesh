// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! X.509 parsing helpers for the `trustchain.*` IPC handlers.
//!
//! Kept in a sibling module so the handler file stays under the 500-line
//! limit. Pure functions, no side effects, no I/O.

use x509_parser::extensions::{ExtendedKeyUsage, GeneralName, KeyUsage};

/// Resolve an OID (as its `String` representation) to a human-readable name.
///
/// Covers standard signature/public-key algorithms, standard X.509
/// extensions, and HyperMesh's custom post-quantum extension OIDs (matching
/// `trustchain::crypto::certificate`).
pub fn oid_to_name(oid: &str) -> Option<&'static str> {
    match oid {
        // Signature / public-key algorithms (RFC 8017 / NIST)
        "1.2.840.113549.1.1.1" => Some("RSA"),
        "1.2.840.113549.1.1.5" => Some("RSA-SHA1"),
        "1.2.840.113549.1.1.11" => Some("RSA-SHA256"),
        "1.2.840.113549.1.1.12" => Some("RSA-SHA384"),
        "1.2.840.113549.1.1.13" => Some("RSA-SHA512"),
        "1.2.840.10045.2.1" => Some("EC-PublicKey"),
        "1.2.840.10045.4.3.2" => Some("ECDSA-SHA256"),
        "1.2.840.10045.4.3.3" => Some("ECDSA-SHA384"),
        "1.2.840.10045.4.3.4" => Some("ECDSA-SHA512"),
        "1.3.101.112" => Some("Ed25519"),
        "1.3.101.113" => Some("Ed448"),

        // Extended Key Usage (RFC 5280 §4.2.1.12)
        "2.5.29.37.0" => Some("any_extended_key_usage"),
        "1.3.6.1.5.5.7.3.1" => Some("server_auth"),
        "1.3.6.1.5.5.7.3.2" => Some("client_auth"),
        "1.3.6.1.5.5.7.3.3" => Some("code_signing"),
        "1.3.6.1.5.5.7.3.4" => Some("email_protection"),
        "1.3.6.1.5.5.7.3.8" => Some("time_stamping"),
        "1.3.6.1.5.5.7.3.9" => Some("ocsp_signing"),

        // Standard X.509 extensions (RFC 5280)
        "2.5.29.14" => Some("subject_key_identifier"),
        "2.5.29.15" => Some("key_usage"),
        "2.5.29.17" => Some("subject_alt_name"),
        "2.5.29.18" => Some("issuer_alt_name"),
        "2.5.29.19" => Some("basic_constraints"),
        "2.5.29.20" => Some("crl_number"),
        "2.5.29.21" => Some("reason_code"),
        "2.5.29.27" => Some("delta_crl_indicator"),
        "2.5.29.28" => Some("issuing_distribution_point"),
        "2.5.29.29" => Some("certificate_issuer"),
        "2.5.29.30" => Some("name_constraints"),
        "2.5.29.31" => Some("crl_distribution_points"),
        "2.5.29.32" => Some("certificate_policies"),
        "2.5.29.33" => Some("policy_mappings"),
        "2.5.29.35" => Some("authority_key_identifier"),
        "2.5.29.36" => Some("policy_constraints"),
        "2.5.29.37" => Some("extended_key_usage"),
        "2.5.29.46" => Some("freshest_crl"),
        "2.5.29.54" => Some("inhibit_any_policy"),
        "1.3.6.1.5.5.7.1.1" => Some("authority_info_access"),
        "1.3.6.1.5.5.7.1.11" => Some("subject_info_access"),

        // HyperMesh post-quantum extension OIDs
        // (see trustchain::crypto::certificate::PQCertificateManager)
        "1.3.6.1.4.1.99999.1" => Some("FALCON-1024"),
        "1.3.6.1.4.1.99999.2" => Some("Kyber-1024"),

        _ => None,
    }
}

/// Format a unix timestamp as RFC 3339 (UTC, seconds precision).
pub fn unix_to_rfc3339(secs: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// Collect KeyUsage flags into a flat list of snake_case names.
pub fn collect_key_usage(ku: &KeyUsage) -> Vec<&'static str> {
    let mut out = Vec::new();
    if ku.digital_signature() {
        out.push("digital_signature");
    }
    if ku.non_repudiation() {
        out.push("non_repudiation");
    }
    if ku.key_encipherment() {
        out.push("key_encipherment");
    }
    if ku.data_encipherment() {
        out.push("data_encipherment");
    }
    if ku.key_agreement() {
        out.push("key_agreement");
    }
    if ku.key_cert_sign() {
        out.push("key_cert_sign");
    }
    if ku.crl_sign() {
        out.push("crl_sign");
    }
    if ku.encipher_only() {
        out.push("encipher_only");
    }
    if ku.decipher_only() {
        out.push("decipher_only");
    }
    out
}

/// Collect ExtendedKeyUsage values into a list of resolved names / OIDs.
pub fn collect_extended_key_usage(eku: &ExtendedKeyUsage<'_>) -> Vec<String> {
    let mut out = Vec::new();
    if eku.any {
        out.push("any_extended_key_usage".to_string());
    }
    if eku.server_auth {
        out.push("server_auth".to_string());
    }
    if eku.client_auth {
        out.push("client_auth".to_string());
    }
    if eku.code_signing {
        out.push("code_signing".to_string());
    }
    if eku.email_protection {
        out.push("email_protection".to_string());
    }
    if eku.time_stamping {
        out.push("time_stamping".to_string());
    }
    if eku.ocsp_signing {
        out.push("ocsp_signing".to_string());
    }
    for oid in &eku.other {
        let s = oid.to_string();
        out.push(oid_to_name(&s).map(String::from).unwrap_or(s));
    }
    out
}

/// Render a single SAN `GeneralName` to a typed string (e.g. `"DNS:example.com"`).
pub fn render_general_name(gn: &GeneralName<'_>) -> String {
    match gn {
        GeneralName::DNSName(s) => format!("DNS:{}", s),
        GeneralName::RFC822Name(s) => format!("email:{}", s),
        GeneralName::URI(s) => format!("URI:{}", s),
        GeneralName::IPAddress(bytes) => render_ip(bytes),
        GeneralName::DirectoryName(dn) => format!("DirName:{}", dn),
        GeneralName::OtherName(oid, _) => {
            let s = oid.to_string();
            format!(
                "OtherName:{}",
                oid_to_name(&s).map(String::from).unwrap_or(s),
            )
        }
        GeneralName::RegisteredID(oid) => {
            let s = oid.to_string();
            format!(
                "RegisteredID:{}",
                oid_to_name(&s).map(String::from).unwrap_or(s),
            )
        }
        GeneralName::X400Address(_) => "X400Address:<unparsed>".to_string(),
        GeneralName::EDIPartyName(_) => "EDIPartyName:<unparsed>".to_string(),
    }
}

fn render_ip(bytes: &[u8]) -> String {
    match bytes.len() {
        4 => format!("IP:{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]),
        16 => {
            let mut groups = [0u16; 8];
            for (i, g) in groups.iter_mut().enumerate() {
                *g = u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
            }
            format!(
                "IP:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                groups[0], groups[1], groups[2], groups[3],
                groups[4], groups[5], groups[6], groups[7],
            )
        }
        _ => format!("IP:<{} bytes>", bytes.len()),
    }
}

/// Parse a DER-encoded X.509 certificate and return a fully-populated JSON
/// object with subject/issuer/validity/algorithm/usage/SAN/extension data.
///
/// Returns `Err(message)` if the DER cannot be parsed. Never panics.
pub fn parse_cert_to_json(
    der: &[u8],
    path: &str,
) -> Result<serde_json::Value, String> {
    let (_, cert) = x509_parser::parse_x509_certificate(der)
        .map_err(|e| format!("parse failed: {e}"))?;

    let tbs = cert.tbs_certificate;

    // Identity / hashes
    let blake3_hex = blake3::hash(der).to_hex().to_string();
    let sha256_hex = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(der))
    };

    // Validity window + active/expired status
    let not_before_ts = tbs.validity.not_before.timestamp();
    let not_after_ts = tbs.validity.not_after.timestamp();
    let now = chrono::Utc::now().timestamp();
    let status = if now < not_before_ts {
        "not_yet_valid"
    } else if now > not_after_ts {
        "expired"
    } else {
        "active"
    };

    // Algorithm OIDs → human names (fallback to OID string).
    let sig_alg_oid = tbs.signature.algorithm.to_string();
    let sig_alg_name = oid_to_name(&sig_alg_oid)
        .map(String::from)
        .unwrap_or_else(|| sig_alg_oid.clone());
    let pk_alg_oid = tbs.subject_pki.algorithm.algorithm.to_string();
    let pk_alg_name = oid_to_name(&pk_alg_oid)
        .map(String::from)
        .unwrap_or_else(|| pk_alg_oid.clone());

    // Key usage (may be absent)
    let key_usage: Vec<&str> = match tbs.key_usage() {
        Ok(Some(ext)) => collect_key_usage(ext.value),
        _ => Vec::new(),
    };

    // Extended key usage
    let extended_key_usage: Vec<String> = match tbs.extended_key_usage() {
        Ok(Some(ext)) => collect_extended_key_usage(ext.value),
        _ => Vec::new(),
    };

    // Subject Alternative Names
    let subject_alt_names: Vec<String> = match tbs.subject_alternative_name() {
        Ok(Some(ext)) => ext
            .value
            .general_names
            .iter()
            .map(render_general_name)
            .collect(),
        _ => Vec::new(),
    };

    // Raw extension list (every entry, with critical flag + resolved name)
    let extensions: Vec<serde_json::Value> = tbs
        .extensions()
        .iter()
        .map(|ext| {
            let oid_str = ext.oid.to_string();
            let name = oid_to_name(&oid_str);
            serde_json::json!({
                "oid": oid_str,
                "critical": ext.critical,
                "name": name,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "id": blake3_hex,
        "subject": tbs.subject.to_string(),
        "issuer": tbs.issuer.to_string(),
        "valid_from": unix_to_rfc3339(not_before_ts),
        "valid_to": unix_to_rfc3339(not_after_ts),
        "status": status,
        "serial_number": hex::encode(tbs.raw_serial()),
        "signature_algorithm": sig_alg_name,
        "signature_algorithm_oid": sig_alg_oid,
        "key_algorithm": pk_alg_name,
        "key_algorithm_oid": pk_alg_oid,
        "fingerprint_sha256": sha256_hex,
        "fingerprint_blake3": blake3_hex,
        "key_usage": key_usage,
        "extended_key_usage": extended_key_usage,
        "subject_alt_names": subject_alt_names,
        "extensions": extensions,
        "path": path,
    }))
}
