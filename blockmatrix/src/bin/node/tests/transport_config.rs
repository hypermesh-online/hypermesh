// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

use super::{node_kernel_acceleration_allowed, stoq_listener_config, uses_physical_interface};
use crate::cli::Cli;
use clap::Parser;
use std::net::Ipv6Addr;

#[test]
fn bind_selection_controls_kernel_and_physical_interface() {
    let default = Cli::try_parse_from(["hypermesh"]).unwrap();
    let config = stoq_listener_config(&default);
    assert_eq!(config.bind_address, Ipv6Addr::UNSPECIFIED);
    assert_eq!(config.port, 9292);
    assert!(config.enable_kernel_acceleration);
    assert!(node_kernel_acceleration_allowed(&default));
    assert!(uses_physical_interface(&config));

    let local = Cli::try_parse_from([
        "hypermesh",
        "--stoq-bind-address",
        "::1",
        "--stoq-port",
        "9443",
    ])
    .unwrap();
    let config = stoq_listener_config(&local);
    assert_eq!(config.bind_address, Ipv6Addr::LOCALHOST);
    assert_eq!(config.port, 9443);
    assert!(!config.enable_kernel_acceleration);
    assert!(!node_kernel_acceleration_allowed(&local));
    assert!(!uses_physical_interface(&config));
    assert!(config.interface.is_none());

    let remote =
        Cli::try_parse_from(["hypermesh", "--stoq-bind-address", "2001:db8::1234"]).unwrap();
    let config = stoq_listener_config(&remote);
    assert_eq!(
        config.bind_address,
        "2001:db8::1234".parse::<Ipv6Addr>().unwrap()
    );
    assert!(config.enable_kernel_acceleration);
    assert!(node_kernel_acceleration_allowed(&remote));
    assert!(uses_physical_interface(&config));
}
