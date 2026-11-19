// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Plain QUIC with standard TLS verification, bound only to IPv6 loopback.
//! Not STOQ, post-quantum transport, CA enrollment, or mesh authorization.
//!
//! Build with `--features future-tests`. Split-process usage:
//! `quic_basic server /caller/controlled/new-public.der`
//! `quic_basic client /caller/controlled/new-public.der`
//! Start the client after the server prints its listening message. The caller must
//! securely transfer/trust this public DER pin and control its directory. The server
//! exclusively creates it (never overwrites/deletes); use a new path each server run.
//! No private key is exported, and paths under /tmp are refused.
//! Uses [::1]:5000; exchange errors exit nonzero.
#![cfg(feature = "future-tests")]

mod quic_support;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    quic_support::run(false).await
}
