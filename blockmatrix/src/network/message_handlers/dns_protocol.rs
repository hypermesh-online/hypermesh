// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase H.1 — distributed DNS query/response wire format.
//!
//! Extends the legacy `TAG_DNS_RESOLVE`/`TAG_DNS_RESOLVE_RESPONSE`
//! (0x09/0x0A) with a richer query that returns enough metadata for
//! conflict resolution across nodes:
//!
//! ```text
//! +-------------------+-------------------+-------+--------+
//! | foundation_grant  | registration ts   | chain | order  |
//! | present (bool)    | (oldest wins)     | height|        |
//! +-------------------+-------------------+-------+--------+
//! ```
//!
//! Conflict winner is the tuple `(foundation_grant_present DESC,
//! registration_timestamp ASC, chain_height DESC)` — foundation grant
//! beats anything else; among equals the older registration wins;
//! tiebreak by higher chain height.
//!
//! This module defines only the wire types. The handler lives in
//! `sync_and_reflection.rs` and the resolver glue lives in `dns/`.

use crate::dns::DnsRecord;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use uuid::Uuid;

/// Distributed DNS query — broadcast to peers when local resolution
/// misses or when the resolver wants to confirm a conflict-resolved
/// canonical answer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedDnsQuery {
    /// Correlation identifier for matching responses to queries.
    pub query_id: Uuid,
    /// Fully-qualified domain name being resolved.
    pub domain_name: String,
}

/// Distributed DNS response — peers reply with the metadata needed for
/// conflict resolution alongside the records themselves.
///
/// `records` may be empty when the responding peer has nothing for the
/// queried name; an empty response is still meaningful (it tells the
/// asker the peer is alive and has answered).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedDnsResponse {
    /// Correlation identifier — must match the originating query.
    pub query_id: Uuid,
    /// Domain name the response is for.
    pub domain_name: String,
    /// DNS records the responder holds for this name (may be empty).
    pub records: Vec<DnsRecord>,
    /// 32-byte BLAKE3 chain identifier the registration lives on.
    /// Hex-encoded for wire stability and human inspection.
    pub chain_id: String,
    /// Block height at which the registration was committed.
    pub chain_height: u64,
    /// Unix-seconds timestamp of the original registration.  Used as
    /// the primary tiebreaker (oldest registration wins).
    pub registration_timestamp: u64,
    /// Whether a foundation-signed grant accompanied the registration.
    /// Foundation-grant entries beat ungrant'd entries unconditionally.
    pub foundation_grant_present: bool,
}

impl DistributedDnsResponse {
    /// Compare two responses by canonical-winner ordering.
    ///
    /// Returns `Ordering::Less` when `self` is the "smaller" (less
    /// canonical) response, `Greater` when more canonical. Sorting a
    /// list with this comparator and taking the *last* element gives
    /// the canonical answer.
    ///
    /// The tuple is:
    ///   1. `foundation_grant_present` — true wins
    ///   2. `registration_timestamp` — *smaller* (older) wins
    ///   3. `chain_height` — larger wins
    pub fn canonical_cmp(&self, other: &Self) -> Ordering {
        match self
            .foundation_grant_present
            .cmp(&other.foundation_grant_present)
        {
            Ordering::Equal => {}
            non_eq => return non_eq,
        }

        // Older registration wins → smaller timestamp is "more canonical"
        // → reverse ordering so the smaller timestamp sorts greater.
        match other
            .registration_timestamp
            .cmp(&self.registration_timestamp)
        {
            Ordering::Equal => {}
            non_eq => return non_eq,
        }

        // Higher chain height wins on ties.
        self.chain_height.cmp(&other.chain_height)
    }
}

/// Selects the canonical winner from a non-empty slice of responses.
///
/// Returns `None` when the slice is empty. Empty-records responses are
/// included in the tiebreak — they only "win" when no peer has any
/// record at all, in which case the resolver returns "not found".
pub fn select_canonical(responses: &[DistributedDnsResponse]) -> Option<&DistributedDnsResponse> {
    responses.iter().max_by(|a, b| a.canonical_cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::{DnsRecordData, DnsRecordType};
    use std::net::Ipv6Addr;
    use std::time::SystemTime;

    fn mk_response(
        ts: u64,
        height: u64,
        grant: bool,
        records: usize,
    ) -> DistributedDnsResponse {
        let recs = (0..records)
            .map(|i| DnsRecord {
                domain: "test".into(),
                record_type: DnsRecordType::AAAA,
                data: DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
                ttl: 300,
                created_at: SystemTime::now(),
                expires_at: SystemTime::now(),
                owner: format!("owner-{i}"),
                tx_hash: None,
            })
            .collect();
        DistributedDnsResponse {
            query_id: Uuid::new_v4(),
            domain_name: "test".into(),
            records: recs,
            chain_id: format!("{:064x}", height),
            chain_height: height,
            registration_timestamp: ts,
            foundation_grant_present: grant,
        }
    }

    #[test]
    fn foundation_grant_beats_no_grant() {
        let no_grant = mk_response(100, 50, false, 1);
        let with_grant = mk_response(200, 1, true, 1); // newer, lower height
        assert_eq!(
            with_grant.canonical_cmp(&no_grant),
            Ordering::Greater,
            "foundation grant must win even when newer / lower height"
        );
    }

    #[test]
    fn older_registration_wins_tiebreak() {
        let older = mk_response(100, 5, false, 1);
        let newer = mk_response(200, 5, false, 1);
        assert_eq!(older.canonical_cmp(&newer), Ordering::Greater);
    }

    #[test]
    fn higher_chain_height_wins_when_timestamps_equal() {
        let lower_height = mk_response(100, 5, false, 1);
        let higher_height = mk_response(100, 50, false, 1);
        assert_eq!(higher_height.canonical_cmp(&lower_height), Ordering::Greater);
    }

    #[test]
    fn select_canonical_picks_grant_winner() {
        let responses = vec![
            mk_response(100, 50, false, 1),
            mk_response(50, 1, true, 1), // grant
            mk_response(75, 100, false, 1),
        ];
        let winner = select_canonical(&responses).expect("test: non-empty");
        assert!(winner.foundation_grant_present);
    }

    #[test]
    fn select_canonical_empty_returns_none() {
        let responses: Vec<DistributedDnsResponse> = vec![];
        assert!(select_canonical(&responses).is_none());
    }

    #[test]
    fn select_canonical_picks_oldest_among_unsigned() {
        let responses = vec![
            mk_response(300, 5, false, 1),
            mk_response(100, 5, false, 1),
            mk_response(200, 5, false, 1),
        ];
        let winner = select_canonical(&responses).expect("test: non-empty");
        assert_eq!(winner.registration_timestamp, 100);
    }
}
