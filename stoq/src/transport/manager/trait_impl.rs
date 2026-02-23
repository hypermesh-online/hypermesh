// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport trait implementation

use anyhow::Result;
use async_trait::async_trait;

use crate::transport::connection::{Connection, Endpoint};

use super::StoqTransport;

#[async_trait]
impl crate::Transport for StoqTransport {
    async fn connect(&self, endpoint: &Endpoint) -> Result<Connection> {
        Ok((*self.connect(endpoint).await?).clone())
    }

    async fn accept(&self) -> Result<Connection> {
        Ok((*self.accept().await?).clone())
    }

    fn stats(&self) -> crate::TransportStats {
        self.stats()
    }

    async fn shutdown(&self) {
        self.shutdown().await
    }
}
