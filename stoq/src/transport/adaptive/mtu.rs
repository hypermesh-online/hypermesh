// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::time::{Duration, Instant};
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MtuProbeState {
    Searching,
    Confirmed,
    Failed,
}

pub struct MtuDiscovery {
    current_mtu: u16,
    min_mtu: u16,
    max_mtu: u16,
    search_high: u16,
    search_low: u16,
    probe_state: MtuProbeState,
    last_probe: Instant,
    probe_interval: Duration,
    /// When true, clamps MTU bounds to internet-safe values (1200–1500).
    /// When false, allows probing up to jumbo frame sizes (9000).
    wan_mode: bool,
}

impl MtuDiscovery {
    pub fn new(min_mtu: u16, max_mtu: u16, probe_interval: Duration) -> Self {
        let min = min_mtu.max(1200);
        let max = max_mtu.max(min);
        Self {
            current_mtu: min,
            min_mtu: min,
            max_mtu: max,
            search_low: min,
            search_high: max,
            probe_state: MtuProbeState::Searching,
            last_probe: Instant::now() - probe_interval,
            probe_interval,
            wan_mode: false,
        }
    }

    /// Enable or disable WAN mode.
    ///
    /// In WAN mode, the MTU search bounds are clamped to internet-safe
    /// values: minimum 1200 bytes, maximum 1500 bytes (standard Ethernet).
    /// In LAN mode (default), probing can reach jumbo frame sizes (9000).
    pub fn set_wan_mode(&mut self, wan: bool) {
        self.wan_mode = wan;
        if wan {
            self.min_mtu = self.min_mtu.max(1200);
            self.max_mtu = self.max_mtu.min(1500);
            // Ensure max >= min after clamping
            if self.max_mtu < self.min_mtu {
                self.max_mtu = self.min_mtu;
            }
            self.search_low = self.search_low.max(self.min_mtu);
            self.search_high = self.search_high.min(self.max_mtu);
            if self.current_mtu > self.max_mtu {
                self.current_mtu = self.max_mtu;
            }
            info!("MTU discovery: WAN mode enabled (bounds {}-{} bytes)", self.min_mtu, self.max_mtu);
        } else {
            info!("MTU discovery: LAN mode (jumbo frames allowed)");
        }
    }

    pub fn should_probe(&self) -> bool {
        self.probe_state == MtuProbeState::Searching
            && self.last_probe.elapsed() >= self.probe_interval
    }

    pub fn next_probe_size(&self) -> u16 {
        let mid = self.search_low + (self.search_high - self.search_low) / 2;
        mid.max(self.min_mtu)
    }

    pub fn probe_succeeded(&mut self, mtu: u16) {
        self.last_probe = Instant::now();
        self.current_mtu = mtu;
        self.search_low = mtu;

        if mtu >= self.max_mtu {
            self.current_mtu = self.max_mtu;
            self.probe_state = MtuProbeState::Confirmed;
            info!("MTU discovery confirmed at {} bytes", self.current_mtu);
        } else if self.search_high - self.search_low <= 1 {
            self.probe_state = MtuProbeState::Confirmed;
            info!("MTU discovery converged at {} bytes", self.current_mtu);
        } else {
            debug!(
                "MTU probe succeeded at {}, searching [{}, {}]",
                mtu, self.search_low, self.search_high
            );
        }
    }

    pub fn probe_failed(&mut self) {
        self.last_probe = Instant::now();
        let mid = self.search_low + (self.search_high - self.search_low) / 2;
        self.search_high = mid;

        if self.search_high <= self.search_low || self.search_high <= self.min_mtu {
            self.current_mtu = self.min_mtu;
            self.probe_state = MtuProbeState::Failed;
            info!(
                "MTU discovery failed, fell back to minimum {} bytes",
                self.min_mtu
            );
        } else {
            debug!(
                "MTU probe failed, narrowing to [{}, {}]",
                self.search_low, self.search_high
            );
        }
    }

    pub fn current_mtu(&self) -> u16 {
        self.current_mtu
    }

    pub fn state(&self) -> &MtuProbeState {
        &self.probe_state
    }

    pub fn reset(&mut self) {
        self.current_mtu = self.min_mtu;
        self.search_low = self.min_mtu;
        self.search_high = self.max_mtu;
        self.probe_state = MtuProbeState::Searching;
        self.last_probe = Instant::now() - self.probe_interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtu_probe_search_confirm() {
        let mut mtu = MtuDiscovery::new(1200, 9000, Duration::from_millis(0));

        for _ in 0..20 {
            if *mtu.state() == MtuProbeState::Confirmed {
                break;
            }
            let probe = mtu.next_probe_size();
            mtu.probe_succeeded(probe);
        }

        assert_eq!(*mtu.state(), MtuProbeState::Confirmed);
        assert!(
            mtu.current_mtu() >= 8900,
            "expected near-max MTU, got {}",
            mtu.current_mtu()
        );
    }

    #[test]
    fn test_mtu_probe_search_fail() {
        let mut mtu = MtuDiscovery::new(1200, 9000, Duration::from_millis(0));

        for _ in 0..20 {
            if *mtu.state() == MtuProbeState::Failed {
                break;
            }
            mtu.probe_failed();
        }

        assert_eq!(*mtu.state(), MtuProbeState::Failed);
        assert_eq!(mtu.current_mtu(), 1200);
    }

    #[test]
    fn test_mtu_probe_binary_search() {
        let mut mtu = MtuDiscovery::new(1200, 9000, Duration::from_millis(0));

        let first_mid = mtu.next_probe_size();
        mtu.probe_succeeded(first_mid);
        assert!(mtu.current_mtu() >= 1200);

        let second_mid = mtu.next_probe_size();
        assert!(second_mid > first_mid, "next probe should be higher");
        mtu.probe_failed();

        let third_probe = mtu.next_probe_size();
        assert!(
            third_probe >= first_mid && third_probe <= second_mid,
            "binary search should narrow: {third_probe} in [{first_mid}, {second_mid}]"
        );
    }
}
