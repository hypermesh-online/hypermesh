// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use crate::transport::{CongestionControl, NetworkTier};

pub fn congestion_control_for_tier(tier: &NetworkTier) -> CongestionControl {
    match tier {
        NetworkTier::Performance { .. }
        | NetworkTier::Enterprise { .. }
        | NetworkTier::DataCenter { .. } => CongestionControl::Bbr2,
        NetworkTier::Standard { .. } | NetworkTier::Home { .. } => CongestionControl::Cubic,
        NetworkTier::MinSpec { .. } | NetworkTier::Slow { .. } => CongestionControl::NewReno,
    }
}

pub(crate) fn tier_step_down(tier: &NetworkTier) -> NetworkTier {
    match tier {
        NetworkTier::DataCenter { .. } => NetworkTier::Enterprise { gbps: 10.0 },
        NetworkTier::Enterprise { .. } => NetworkTier::Performance { gbps: 2.5 },
        NetworkTier::Performance { .. } => NetworkTier::Standard { gbps: 1.0 },
        NetworkTier::Standard { .. } => NetworkTier::Home { mbps: 100.0 },
        NetworkTier::Home { .. } | NetworkTier::Slow { .. } => NetworkTier::Slow { mbps: 10.0 },
        NetworkTier::MinSpec { .. } => NetworkTier::MinSpec { mbps: 1.0 },
    }
}

pub(crate) fn tier_step_up(tier: &NetworkTier) -> NetworkTier {
    match tier {
        NetworkTier::MinSpec { .. } => NetworkTier::Slow { mbps: 10.0 },
        NetworkTier::Slow { .. } => NetworkTier::Home { mbps: 100.0 },
        NetworkTier::Home { .. } => NetworkTier::Standard { gbps: 1.0 },
        NetworkTier::Standard { .. } => NetworkTier::Performance { gbps: 2.5 },
        NetworkTier::Performance { .. } => NetworkTier::Enterprise { gbps: 10.0 },
        NetworkTier::Enterprise { .. } | NetworkTier::DataCenter { .. } => {
            NetworkTier::DataCenter { gbps: 25.0 }
        }
    }
}

pub(crate) fn tiers_equal(a: &NetworkTier, b: &NetworkTier) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cc_tier_mapping() {
        assert!(matches!(
            congestion_control_for_tier(&NetworkTier::DataCenter { gbps: 40.0 }),
            CongestionControl::Bbr2
        ));
        assert!(matches!(
            congestion_control_for_tier(&NetworkTier::Standard { gbps: 1.0 }),
            CongestionControl::Cubic
        ));
        assert!(matches!(
            congestion_control_for_tier(&NetworkTier::Slow { mbps: 10.0 }),
            CongestionControl::NewReno
        ));
    }

    #[test]
    fn test_tier_step_down_floor() {
        let slow = NetworkTier::Slow { mbps: 10.0 };
        let result = tier_step_down(&slow);
        assert!(matches!(result, NetworkTier::Slow { .. }));
    }

    #[test]
    fn test_tier_step_up_ceiling() {
        let dc = NetworkTier::DataCenter { gbps: 40.0 };
        let result = tier_step_up(&dc);
        assert!(matches!(result, NetworkTier::DataCenter { .. }));
    }
}
