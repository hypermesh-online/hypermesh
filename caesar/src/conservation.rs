// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Conservation Law (Whitepaper section 3.1)
//!
//! Enforces the thermodynamic invariant of the Caesar EVP system:
//!
//!     Input Value = Output Value + Transit Fees + Demurrage Decay
//!
//! Every settlement epoch, the conservation law is verified across all
//! packet and settlement records. A circuit breaker halts minting if
//! the cumulative conservation error exceeds the configured threshold.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use hypermesh_lib::economic::{GoldGrams, PacketState};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors raised when conservation invariants are violated.
#[derive(Debug, thiserror::Error)]
pub enum ConservationError {
    #[error("circuit breaker tripped — minting halted (cumulative error: {0})")]
    CircuitBreakerTripped(Decimal),

    #[error("settlement imbalance: expected {expected}, got {actual}")]
    SettlementImbalance { expected: Decimal, actual: Decimal },
}

// ---------------------------------------------------------------------------
// Conservation Law
// ---------------------------------------------------------------------------

/// Tracks cumulative conservation error and the circuit breaker state.
///
/// Each call to [`verify_settlement`](Self::verify_settlement) accumulates
/// the absolute error of one settlement. When the cumulative error exceeds
/// `circuit_breaker_threshold`, the breaker trips and all further settlements
/// are rejected until an administrator calls [`reset_circuit_breaker`](Self::reset_circuit_breaker).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationLaw {
    cumulative_error: Decimal,
    circuit_breaker_threshold: Decimal,
    circuit_breaker_tripped: bool,
    last_audit: DateTime<Utc>,
}

/// Per-settlement tolerance for tiny rounding differences.
const SETTLEMENT_TOLERANCE: Decimal = dec!(0.0001);

impl ConservationLaw {
    /// Create a new conservation law tracker with the given circuit breaker
    /// threshold. The default threshold is `0.001`.
    pub fn new(threshold: Decimal) -> Self {
        Self {
            cumulative_error: Decimal::ZERO,
            circuit_breaker_threshold: threshold,
            circuit_breaker_tripped: false,
            last_audit: Utc::now(),
        }
    }

    /// Verify that a single settlement obeys the conservation invariant:
    ///
    ///     initial_value = settled_value + fees + demurrage
    ///
    /// A per-settlement tolerance of `0.0001` absorbs floating-point
    /// rounding. The absolute error is accumulated regardless and the
    /// circuit breaker trips when cumulative error exceeds the threshold.
    pub fn verify_settlement(
        &mut self,
        initial_value: GoldGrams,
        settled_value: GoldGrams,
        fees: GoldGrams,
        demurrage: GoldGrams,
    ) -> Result<(), ConservationError> {
        if self.circuit_breaker_tripped {
            return Err(ConservationError::CircuitBreakerTripped(
                self.cumulative_error,
            ));
        }

        let expected = initial_value.0;
        let actual = settled_value.0 + fees.0 + demurrage.0;
        let error = (expected - actual).abs();

        self.cumulative_error += error;

        if self.cumulative_error > self.circuit_breaker_threshold {
            self.circuit_breaker_tripped = true;
            return Err(ConservationError::CircuitBreakerTripped(
                self.cumulative_error,
            ));
        }

        if error > SETTLEMENT_TOLERANCE {
            return Err(ConservationError::SettlementImbalance { expected, actual });
        }

        Ok(())
    }

    /// Full conservation audit across all stored packets and settlements.
    ///
    /// Reads every packet and settlement record from storage and computes:
    ///
    /// ```text
    /// conservation_error = total_minted
    ///     - (total_settled + total_fees + total_demurrage
    ///        + total_in_flight + total_refunded + total_dissolved)
    /// ```
    pub async fn audit(
        &mut self,
        storage: &crate::storage::CaesarStorage,
    ) -> Result<ConservationAudit, anyhow::Error> {
        let packets = storage.list_all_packets().await?;
        let settlements = storage.list_all_settlements().await?;

        let mut total_minted = GoldGrams::zero();
        let mut total_settled = GoldGrams::zero();
        let mut total_demurrage = GoldGrams::zero();
        let mut total_in_flight = GoldGrams::zero();
        let mut total_refunded = GoldGrams::zero();
        let mut total_dissolved = GoldGrams::zero();

        for packet in &packets {
            total_minted = total_minted + packet.initial_value;
            total_demurrage = total_demurrage + packet.demurrage_cost;

            match packet.state {
                PacketState::Settled => {
                    total_settled = total_settled + packet.current_value;
                }
                PacketState::Refunded => {
                    total_refunded = total_refunded + packet.current_value;
                }
                PacketState::Dissolved => {
                    total_dissolved = total_dissolved + packet.current_value;
                }
                _ if packet.state.is_active() => {
                    total_in_flight = total_in_flight + packet.current_value;
                }
                _ => {}
            }
        }

        let total_fees = settlements
            .iter()
            .fold(GoldGrams::zero(), |acc, s| acc + s.fee_collected);

        let conservation_error = total_minted.0
            - (total_settled.0
                + total_fees.0
                + total_demurrage.0
                + total_in_flight.0
                + total_refunded.0
                + total_dissolved.0);

        let is_balanced = conservation_error.abs() < dec!(0.001);

        self.last_audit = Utc::now();

        Ok(ConservationAudit {
            total_minted,
            total_settled,
            total_fees,
            total_demurrage,
            total_in_flight,
            total_refunded,
            total_dissolved,
            conservation_error,
            is_balanced,
            audited_at: self.last_audit,
        })
    }

    /// Whether the circuit breaker is currently tripped.
    pub fn is_circuit_breaker_tripped(&self) -> bool {
        self.circuit_breaker_tripped
    }

    /// Admin reset after investigation clears the breaker and error.
    pub fn reset_circuit_breaker(&mut self) {
        self.circuit_breaker_tripped = false;
        self.cumulative_error = Decimal::ZERO;
    }
}

// ---------------------------------------------------------------------------
// Conservation Audit
// ---------------------------------------------------------------------------

/// Result of a full conservation audit across all stored records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationAudit {
    pub total_minted: GoldGrams,
    pub total_settled: GoldGrams,
    pub total_fees: GoldGrams,
    pub total_demurrage: GoldGrams,
    pub total_in_flight: GoldGrams,
    pub total_refunded: GoldGrams,
    pub total_dissolved: GoldGrams,
    pub conservation_error: Decimal,
    pub is_balanced: bool,
    pub audited_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn gg(n: i64, scale: u32) -> GoldGrams {
        GoldGrams::from_decimal(Decimal::new(n, scale))
    }

    #[test]
    fn verify_settlement_balanced() {
        let mut law = ConservationLaw::new(dec!(0.001));
        let result = law.verify_settlement(gg(100, 0), gg(95, 0), gg(3, 0), gg(2, 0));
        assert!(result.is_ok(), "balanced settlement should pass");
    }

    #[test]
    fn verify_settlement_imbalanced() {
        // Threshold high enough that the circuit breaker does NOT trip,
        // so the SettlementImbalance error is returned instead.
        let mut law = ConservationLaw::new(dec!(100.0));
        let result = law.verify_settlement(gg(100, 0), gg(90, 0), gg(3, 0), gg(2, 0));
        assert!(result.is_err(), "imbalanced settlement should fail");
        let err = result.expect_err("test: should be SettlementImbalance");
        assert!(
            matches!(err, ConservationError::SettlementImbalance { .. }),
            "expected SettlementImbalance, got: {err}"
        );
    }

    #[test]
    fn circuit_breaker_trips_on_threshold() {
        let mut law = ConservationLaw::new(dec!(0.001));
        // Each settlement has error of 5 (100 - 90 - 3 - 2 = 5).
        // Threshold 0.001 should trip on the first one.
        let result = law.verify_settlement(gg(100, 0), gg(90, 0), gg(3, 0), gg(2, 0));
        assert!(result.is_err());
        assert!(law.is_circuit_breaker_tripped());
    }

    #[test]
    fn circuit_breaker_blocks_after_trip() {
        let mut law = ConservationLaw::new(dec!(0.001));
        // Trip it
        let _ = law.verify_settlement(gg(100, 0), gg(90, 0), gg(3, 0), gg(2, 0));
        assert!(law.is_circuit_breaker_tripped());

        // Subsequent balanced settlement should still be rejected
        let result = law.verify_settlement(gg(100, 0), gg(95, 0), gg(3, 0), gg(2, 0));
        assert!(result.is_err());
        let err = result.expect_err("test: should be CircuitBreakerTripped");
        assert!(
            matches!(err, ConservationError::CircuitBreakerTripped(_)),
            "expected CircuitBreakerTripped, got: {err}"
        );
    }

    #[test]
    fn reset_circuit_breaker_clears() {
        let mut law = ConservationLaw::new(dec!(0.001));
        let _ = law.verify_settlement(gg(100, 0), gg(90, 0), gg(3, 0), gg(2, 0));
        assert!(law.is_circuit_breaker_tripped());

        law.reset_circuit_breaker();
        assert!(!law.is_circuit_breaker_tripped());

        // Should accept balanced settlements again
        let result = law.verify_settlement(gg(100, 0), gg(95, 0), gg(3, 0), gg(2, 0));
        assert!(result.is_ok(), "reset breaker should allow settlements");
    }

    #[test]
    fn new_default_threshold() {
        let law = ConservationLaw::new(dec!(0.001));
        assert_eq!(law.circuit_breaker_threshold, dec!(0.001));
    }

    #[test]
    fn verify_zero_settlement() {
        let mut law = ConservationLaw::new(dec!(0.001));
        let result = law.verify_settlement(
            GoldGrams::zero(),
            GoldGrams::zero(),
            GoldGrams::zero(),
            GoldGrams::zero(),
        );
        assert!(result.is_ok(), "all-zero settlement should be balanced");
    }

    #[test]
    fn cumulative_error_accumulates() {
        // Threshold high enough to not trip immediately
        let mut law = ConservationLaw::new(dec!(10.0));

        // Error = |100 - (99 + 0 + 0)| = 1
        let _ = law.verify_settlement(gg(100, 0), gg(99, 0), gg(0, 0), gg(0, 0));
        assert_eq!(law.cumulative_error, dec!(1));

        // Error = |200 - (197 + 1 + 1)| = 1
        let _ = law.verify_settlement(gg(200, 0), gg(197, 0), gg(1, 0), gg(1, 0));
        assert_eq!(law.cumulative_error, dec!(2));

        // Error = |50 - (48 + 0 + 0)| = 2
        let _ = law.verify_settlement(gg(50, 0), gg(48, 0), gg(0, 0), gg(0, 0));
        assert_eq!(law.cumulative_error, dec!(4));
    }

    #[test]
    fn verify_settlement_with_rounding() {
        let mut law = ConservationLaw::new(dec!(1.0));
        // Tiny rounding difference: 100 - (99.9999 + 0 + 0) = 0.0001
        // Within SETTLEMENT_TOLERANCE of 0.0001, should pass
        let result = law.verify_settlement(
            gg(1000000, 4), // 100.0000
            gg(999999, 4),  // 99.9999
            GoldGrams::zero(),
            GoldGrams::zero(),
        );
        assert!(result.is_ok(), "tiny rounding should be within tolerance");
    }

    #[test]
    fn circuit_breaker_not_tripped_initially() {
        let law = ConservationLaw::new(dec!(0.001));
        assert!(!law.is_circuit_breaker_tripped());
        assert_eq!(law.cumulative_error, Decimal::ZERO);
    }
}
