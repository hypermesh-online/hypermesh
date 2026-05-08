// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase J.1 — Release feed subscription.
//!
//! The foundation publishes signed [`ReleaseFeedEntry`] records as
//! Catalog assets (`release.feed/v1` typedef). Each subscribing daemon
//! validates the FALCON-1024 signature against a configured foundation
//! public key, then caches the latest entry per channel. The CLI and
//! dashboard query the cache via `system.check_update`.
//!
//! ## Alpha-default inert
//!
//! When `foundation_pubkey` is `None` (the daemon was not configured
//! with a foundation root public key), [`ReleaseFeedSubscriber::ingest`]
//! rejects all entries with [`ReleaseFeedError::NotConfigured`]. This
//! keeps the substrate inert until an operator explicitly opts in.
//!
//! ## Wire format of `signing_payload`
//!
//! `[version UTF-8 || 0x00 || channel || 0x00 ||
//!   canonical(binary_hashes) UTF-8 || 0x00 || issued_at_secs_le]`
//!
//! `canonical(binary_hashes)` is the JSON object serialization of the
//! `binary_hashes` map with keys sorted lexicographically and no
//! whitespace. We use [`serde_json::to_string`] on a [`BTreeMap`] so
//! the same canonicalization is produced on signing and on
//! verification.

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

/// Release channel — clients subscribe per channel and receive only
/// entries that match.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    /// Stable production release.
    Stable,
    /// Beta release — exercised by adventurous operators.
    Beta,
    /// Nightly snapshot — bleeding edge, no stability guarantees.
    Nightly,
}

impl ReleaseChannel {
    /// Wire-encoded channel identifier (lowercase ASCII).
    pub fn as_str(&self) -> &'static str {
        match self {
            ReleaseChannel::Stable => "stable",
            ReleaseChannel::Beta => "beta",
            ReleaseChannel::Nightly => "nightly",
        }
    }

    /// Parse a channel from its string form. Case-insensitive.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "stable" => Some(ReleaseChannel::Stable),
            "beta" => Some(ReleaseChannel::Beta),
            "nightly" => Some(ReleaseChannel::Nightly),
            _ => None,
        }
    }
}

/// A single foundation-signed release feed entry.
///
/// See module docs for canonical signing-payload layout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseFeedEntry {
    /// Semver version string (e.g. "0.2.0", "1.0.0-rc1").
    pub version: String,
    /// Channel this release is published on.
    pub channel: ReleaseChannel,
    /// Per-target binary SHA-256 hashes, keyed by Rust target triple
    /// (e.g. `"x86_64-unknown-linux-musl"`).
    pub binary_hashes: HashMap<String, String>,
    /// Pointer to release notes (HTTPS or HyperMesh DNS asset URL).
    pub release_notes_url: String,
    /// Foundation FALCON-1024 public key bytes.
    pub signed_by: Vec<u8>,
    /// Foundation FALCON-1024 detached signature over `signing_payload`.
    pub signature: Vec<u8>,
    /// Optional minimum client version required to upgrade directly.
    #[serde(default)]
    pub requires_min_version: Option<String>,
    /// Whether this release introduces protocol-breaking changes.
    #[serde(default)]
    pub breaking_changes: bool,
    /// Timestamp the foundation signed this entry.
    pub issued_at: SystemTime,
}

impl ReleaseFeedEntry {
    /// Produce the canonical bytes that are signed by the foundation
    /// and verified on ingest.
    pub fn signing_payload(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            self.version.len() + 1 + self.channel.as_str().len() + 1 + 32 + 1 + 8,
        );
        buf.extend_from_slice(self.version.as_bytes());
        buf.push(0u8);
        buf.extend_from_slice(self.channel.as_str().as_bytes());
        buf.push(0u8);

        // Canonicalize binary_hashes as a sorted-key JSON object so the
        // signing and verifying paths produce byte-identical input.
        let sorted: BTreeMap<&String, &String> = self.binary_hashes.iter().collect();
        let canonical_hashes = serde_json::to_string(&sorted)
            .unwrap_or_else(|_| "{}".to_string());
        buf.extend_from_slice(canonical_hashes.as_bytes());
        buf.push(0u8);
        buf.extend_from_slice(&systime_to_secs_le(self.issued_at));
        buf
    }

    /// Verify the FALCON-1024 signature against `foundation_pubkey`.
    ///
    /// Does NOT check `signed_by` matches `foundation_pubkey` — callers
    /// who want that check should compare those byte slices separately
    /// before calling. The reason for this split: an entry may be
    /// distributed with the issuer pubkey as a cache-key while a
    /// daemon's `foundation_pubkey` is the only authoritative trust
    /// anchor.
    pub fn verify(&self, foundation_pubkey: &[u8]) -> Result<(), ReleaseFeedError> {
        if self.signature.is_empty() {
            return Err(ReleaseFeedError::InvalidSignature);
        }
        let payload = self.signing_payload();
        match <trustchain::FalconIdentity as hypermesh_lib::NodeSigner>::verify_signature(
            foundation_pubkey,
            &payload,
            &self.signature,
        ) {
            Ok(true) => Ok(()),
            _ => Err(ReleaseFeedError::InvalidSignature),
        }
    }
}

fn systime_to_secs_le(t: SystemTime) -> [u8; 8] {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_le_bytes()
}

/// Compare two semver strings as tuples of `(u64, u64, u64)` ignoring
/// any pre-release suffix. Returns:
///   - `std::cmp::Ordering::Less` if `a < b`
///   - `std::cmp::Ordering::Equal` if `a == b` or both are unparseable
///   - `std::cmp::Ordering::Greater` if `a > b`
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> (u64, u64, u64) {
        let core = s.split('-').next().unwrap_or(s);
        let mut parts = core.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
        let major = parts.next().unwrap_or(0);
        let minor = parts.next().unwrap_or(0);
        let patch = parts.next().unwrap_or(0);
        (major, minor, patch)
    };
    parse(a).cmp(&parse(b))
}

/// Errors returned by the release feed subscriber.
#[derive(Debug, Error)]
pub enum ReleaseFeedError {
    /// Signature did not verify against the configured foundation
    /// public key.
    #[error("release feed entry signature failed verification")]
    InvalidSignature,
    /// No foundation public key has been configured on this daemon.
    #[error("release feed subscriber not configured (no foundation pubkey)")]
    NotConfigured,
    /// Entry was rejected because a newer or equal entry already
    /// exists for the same `version`.
    #[error("release feed entry version {0} already cached and not newer")]
    AlreadyCached(String),
}

/// In-memory cache of foundation-signed [`ReleaseFeedEntry`] records.
///
/// Indexed by `version` so multiple channels can be tracked
/// simultaneously. The subscriber is created with no foundation
/// pubkey (`alpha-default inert`); operators opt-in via
/// [`ReleaseFeedSubscriber::set_foundation_pubkey`].
pub struct ReleaseFeedSubscriber {
    foundation_pubkey: RwLock<Option<Vec<u8>>>,
    available_versions: RwLock<HashMap<String, ReleaseFeedEntry>>,
}

impl Default for ReleaseFeedSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl ReleaseFeedSubscriber {
    /// Construct an empty, alpha-default-inert subscriber.
    pub fn new() -> Self {
        Self {
            foundation_pubkey: RwLock::new(None),
            available_versions: RwLock::new(HashMap::new()),
        }
    }

    /// Construct a pre-configured subscriber with the foundation key
    /// already populated. Useful for tests and operator-driven
    /// startup.
    pub fn with_foundation_pubkey(pubkey: Vec<u8>) -> Self {
        Self {
            foundation_pubkey: RwLock::new(Some(pubkey)),
            available_versions: RwLock::new(HashMap::new()),
        }
    }

    /// Configure the foundation public key. Once set, subsequent
    /// [`ReleaseFeedSubscriber::ingest`] calls verify against this key.
    pub async fn set_foundation_pubkey(&self, pubkey: Vec<u8>) {
        let mut guard = self.foundation_pubkey.write().await;
        *guard = Some(pubkey);
    }

    /// Return the configured foundation public key, if any.
    pub async fn foundation_pubkey(&self) -> Option<Vec<u8>> {
        self.foundation_pubkey.read().await.clone()
    }

    /// Ingest a release feed entry. Verifies the FALCON-1024 signature
    /// against the configured foundation pubkey, then inserts it into
    /// the cache if it is new or newer than the existing entry for
    /// that version.
    pub async fn ingest(&self, entry: ReleaseFeedEntry) -> Result<(), ReleaseFeedError> {
        let pubkey = match self.foundation_pubkey.read().await.clone() {
            Some(k) => k,
            None => return Err(ReleaseFeedError::NotConfigured),
        };
        entry.verify(&pubkey)?;

        let mut cache = self.available_versions.write().await;
        if let Some(existing) = cache.get(&entry.version) {
            // Reject if existing entry has a strictly newer issued_at.
            // We accept equal-or-newer to allow re-publication.
            if existing.issued_at > entry.issued_at {
                return Err(ReleaseFeedError::AlreadyCached(entry.version.clone()));
            }
        }
        cache.insert(entry.version.clone(), entry);
        Ok(())
    }

    /// Return the latest entry for the given channel, by semver
    /// ordering of `version` with `issued_at` as tiebreaker.
    pub async fn latest_for_channel(&self, channel: ReleaseChannel) -> Option<ReleaseFeedEntry> {
        let cache = self.available_versions.read().await;
        cache
            .values()
            .filter(|e| e.channel == channel)
            .max_by(|a, b| {
                compare_versions(&a.version, &b.version)
                    .then_with(|| a.issued_at.cmp(&b.issued_at))
            })
            .cloned()
    }

    /// Return all cached entries (any channel), unordered.
    pub async fn all_versions(&self) -> Vec<ReleaseFeedEntry> {
        let cache = self.available_versions.read().await;
        cache.values().cloned().collect()
    }

    /// Number of cached entries (for diagnostics / tests).
    pub async fn cached_count(&self) -> usize {
        self.available_versions.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::NodeSigner;
    use std::time::Duration;
    use trustchain::FalconIdentity;

    fn fixture_entry(
        version: &str,
        channel: ReleaseChannel,
        signer: &FalconIdentity,
    ) -> ReleaseFeedEntry {
        let mut hashes = HashMap::new();
        hashes.insert(
            "x86_64-unknown-linux-musl".to_string(),
            "deadbeefcafef00dba5eba11badf00d0a1b2c3d4e5f60718293a4b5c6d7e8f901".to_string(),
        );
        let mut entry = ReleaseFeedEntry {
            version: version.to_string(),
            channel,
            binary_hashes: hashes,
            release_notes_url: format!("https://release.hypermesh.online/{}", version),
            signed_by: signer.public_key.clone(),
            signature: Vec::new(),
            requires_min_version: None,
            breaking_changes: false,
            issued_at: SystemTime::now(),
        };
        entry.signature = signer
            .sign(&entry.signing_payload())
            .expect("test: FALCON sign");
        entry
    }

    #[test]
    fn signing_payload_is_deterministic() {
        let foundation = FalconIdentity::generate();
        let e = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        let p1 = e.signing_payload();
        let p2 = e.signing_payload();
        assert_eq!(p1, p2);
    }

    #[test]
    fn signing_payload_includes_canonical_hashes() {
        let foundation = FalconIdentity::generate();
        let mut e = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        let p1 = e.signing_payload();

        // Reordering the HashMap insertion shouldn't change the payload
        // — we re-canonicalize on each call.
        let mut new_hashes = HashMap::new();
        new_hashes.insert("zzzz".to_string(), "1".repeat(64));
        new_hashes.insert("aaaa".to_string(), "2".repeat(64));
        e.binary_hashes = new_hashes;
        let p2 = e.signing_payload();
        assert_ne!(p1, p2);
        // Order of inserts doesn't matter:
        let mut other_hashes = HashMap::new();
        other_hashes.insert("aaaa".to_string(), "2".repeat(64));
        other_hashes.insert("zzzz".to_string(), "1".repeat(64));
        e.binary_hashes = other_hashes;
        let p3 = e.signing_payload();
        assert_eq!(p2, p3);
    }

    #[tokio::test]
    async fn verify_succeeds_with_correct_key() {
        let foundation = FalconIdentity::generate();
        let entry = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        assert!(entry.verify(&foundation.public_key).is_ok());
    }

    #[tokio::test]
    async fn verify_fails_with_wrong_key() {
        let foundation = FalconIdentity::generate();
        let other = FalconIdentity::generate();
        let entry = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        assert!(matches!(
            entry.verify(&other.public_key),
            Err(ReleaseFeedError::InvalidSignature)
        ));
    }

    #[tokio::test]
    async fn ingest_requires_foundation_pubkey() {
        let foundation = FalconIdentity::generate();
        let entry = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        let sub = ReleaseFeedSubscriber::new();
        assert!(matches!(
            sub.ingest(entry).await,
            Err(ReleaseFeedError::NotConfigured)
        ));
    }

    #[tokio::test]
    async fn ingest_succeeds_after_configuring_pubkey() {
        let foundation = FalconIdentity::generate();
        let entry = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        let sub = ReleaseFeedSubscriber::new();
        sub.set_foundation_pubkey(foundation.public_key.clone()).await;
        assert!(sub.ingest(entry).await.is_ok());
        assert_eq!(sub.cached_count().await, 1);
    }

    #[tokio::test]
    async fn latest_for_channel_picks_newest_semver() {
        let foundation = FalconIdentity::generate();
        let sub = ReleaseFeedSubscriber::with_foundation_pubkey(foundation.public_key.clone());
        let mut older = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        older.issued_at = SystemTime::now() - Duration::from_secs(60);
        // re-sign with new issued_at
        older.signature = foundation
            .sign(&older.signing_payload())
            .expect("test: sign");
        let newer = fixture_entry("0.2.0", ReleaseChannel::Stable, &foundation);
        sub.ingest(older).await.expect("test: ingest older");
        sub.ingest(newer).await.expect("test: ingest newer");
        let latest = sub
            .latest_for_channel(ReleaseChannel::Stable)
            .await
            .expect("test: latest");
        assert_eq!(latest.version, "0.2.0");
    }

    #[tokio::test]
    async fn latest_for_channel_distinguishes_channels() {
        let foundation = FalconIdentity::generate();
        let sub = ReleaseFeedSubscriber::with_foundation_pubkey(foundation.public_key.clone());
        let stable = fixture_entry("0.1.0", ReleaseChannel::Stable, &foundation);
        let beta = fixture_entry("0.3.0", ReleaseChannel::Beta, &foundation);
        sub.ingest(stable).await.expect("test: stable");
        sub.ingest(beta).await.expect("test: beta");
        let stable_latest = sub
            .latest_for_channel(ReleaseChannel::Stable)
            .await
            .expect("test: stable latest");
        let beta_latest = sub
            .latest_for_channel(ReleaseChannel::Beta)
            .await
            .expect("test: beta latest");
        assert_eq!(stable_latest.version, "0.1.0");
        assert_eq!(beta_latest.version, "0.3.0");
    }

    #[test]
    fn compare_versions_orders_correctly() {
        use std::cmp::Ordering::*;
        assert_eq!(compare_versions("0.1.0", "0.2.0"), Less);
        assert_eq!(compare_versions("1.0.0", "0.9.9"), Greater);
        assert_eq!(compare_versions("0.1.0", "0.1.0"), Equal);
        assert_eq!(compare_versions("1.0.0-rc1", "1.0.0"), Equal); // pre-release ignored for ordering
        assert_eq!(compare_versions("0.10.0", "0.9.0"), Greater);
    }
}
