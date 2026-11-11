// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Parser-only STOQ listener selection controls; no socket is opened here.

use super::{Cli, PrivacyModeArg};
use clap::Parser;
use std::net::Ipv6Addr;

#[test]
fn stoq_bind_defaults_to_all_ipv6_interfaces() {
    let cli = Cli::try_parse_from(["hypermesh"]).expect("default CLI parses");
    assert_eq!(cli.stoq_bind_address, Ipv6Addr::UNSPECIFIED);
    assert_eq!(cli.stoq_port, 9292);
}

#[test]
fn stoq_bind_accepts_exact_ipv6_and_preserves_other_flags() {
    let loopback = Cli::try_parse_from(["hypermesh", "--stoq-bind-address", "::1"])
        .expect("IPv6 loopback parses");
    assert_eq!(loopback.stoq_bind_address, Ipv6Addr::LOCALHOST);

    let cli = Cli::try_parse_from([
        "hypermesh",
        "--stoq-bind-address",
        "2001:db8::1234",
        "--stoq-port",
        "9443",
        "--privacy",
        "public",
        "--bootstrap",
        "2001:db8::9",
    ])
    .expect("explicit IPv6 and preexisting flags parse");
    assert_eq!(
        cli.stoq_bind_address,
        "2001:db8::1234".parse::<Ipv6Addr>().unwrap()
    );
    assert_eq!(cli.stoq_port, 9443);
    assert!(matches!(cli.privacy, PrivacyModeArg::Public));
    assert_eq!(cli.bootstrap, ["2001:db8::9"]);
}

#[test]
fn stoq_bind_rejects_non_ipv6_before_startup() {
    for invalid in ["127.0.0.1", "localhost", "2001:db8::gg", "[::1]:9292", ""] {
        assert!(
            Cli::try_parse_from(["hypermesh", "--stoq-bind-address", invalid]).is_err(),
            "invalid address accepted: {invalid:?}"
        );
    }
}
