// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Caesar EVP IPC handlers: wallet overview, balance, transactions, rewards, staking.
//!
//! When the `caesar` feature is enabled and a `CaesarProtocol` instance is attached
//! to [`DaemonState`], these handlers return live data from the protocol. When the
//! feature is disabled or initialization failed, handlers return an honest
//! `feature_unavailable` error field rather than faking zeros.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::state::DaemonState;

#[cfg(feature = "caesar")]
use rust_decimal::prelude::ToPrimitive;

pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    {
        let s = state.clone();
        handler.register(
            "caesar.overview",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_overview(&s).await })
            }),
        );
    }

    {
        let s = state.clone();
        handler.register(
            "caesar.balance",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_balance(&s).await })
            }),
        );
    }

    {
        let s = state.clone();
        handler.register(
            "caesar.transactions",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_transactions(params, &s).await })
            }),
        );
    }

    {
        let s = state.clone();
        handler.register(
            "caesar.rewards",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_rewards(&s).await })
            }),
        );
    }

    {
        let s = state.clone();
        handler.register(
            "caesar.staking",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_staking(&s).await })
            }),
        );
    }
}

#[cfg(not(feature = "caesar"))]
fn feature_disabled(state: &DaemonState, method: &str) -> serde_json::Value {
    serde_json::json!({
        "node_id": state.node_id,
        "error": "feature_unavailable",
        "method": method,
        "message": "caesar feature not enabled at build time",
    })
}

#[cfg(feature = "caesar")]
fn protocol_unavailable(state: &DaemonState, method: &str) -> serde_json::Value {
    serde_json::json!({
        "node_id": state.node_id,
        "error": "protocol_unavailable",
        "method": method,
        "message": "Caesar protocol failed to initialize at daemon startup",
    })
}

async fn handle_overview(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let chain_height = state.blockchain.get_height().await;
    let uptime = state.started_at.elapsed().as_secs();

    #[cfg(feature = "caesar")]
    {
        let Some(caesar) = state.caesar.as_ref() else {
            let mut v = protocol_unavailable(state, "caesar.overview");
            v["chain_height"] = chain_height.into();
            v["uptime_secs"] = uptime.into();
            return Ok(v);
        };
        let protocol = caesar.read().await;
        let active_packets = protocol.active_packet_count().await.unwrap_or(0);
        let in_transit = protocol
            .in_transit_value()
            .await
            .map(|g| g.0.to_f64().unwrap_or(0.0))
            .unwrap_or(0.0);
        let conservation_tripped = protocol.conservation_status();
        let gold_price_per_gram = {
            let oz_price = protocol.oracle().current_gold_price_usd().await;
            let grams_per_oz = rust_decimal::Decimal::new(311035, 4);
            (oz_price / grams_per_oz).to_f64().unwrap_or(0.0)
        };

        return Ok(serde_json::json!({
            "balance": {
                "total": in_transit,
                "available": in_transit,
                "locked": 0.0,
                "pending": 0.0,
                "staked": 0.0,
            },
            "active_packets": active_packets,
            "pending_rewards": 0.0,
            "chain_height": chain_height,
            "node_id": state.node_id,
            "uptime_secs": uptime,
            "conservation_breaker_tripped": conservation_tripped,
            "gold_price_usd_per_gram": gold_price_per_gram,
            "status": "alpha",
        }));
    }
    #[cfg(not(feature = "caesar"))]
    {
        let mut v = feature_disabled(state, "caesar.overview");
        v["chain_height"] = chain_height.into();
        v["uptime_secs"] = uptime.into();
        Ok(v)
    }
}

async fn handle_balance(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    #[cfg(feature = "caesar")]
    {
        let Some(caesar) = state.caesar.as_ref() else {
            return Ok(protocol_unavailable(state, "caesar.balance"));
        };
        let protocol = caesar.read().await;
        let in_transit = protocol
            .in_transit_value()
            .await
            .map(|g| g.0.to_f64().unwrap_or(0.0))
            .unwrap_or(0.0);
        let gold_per_gram = {
            let oz_price = protocol.oracle().current_gold_price_usd().await;
            let grams_per_oz = rust_decimal::Decimal::new(311035, 4);
            (oz_price / grams_per_oz).to_f64().unwrap_or(0.0)
        };

        return Ok(serde_json::json!({
            "node_id": state.node_id,
            "total": in_transit,
            "available": in_transit,
            "locked": 0.0,
            "pending": 0.0,
            "staked": 0.0,
            "currency": "CAES",
            "gold_backing_grams": in_transit,
            "gold_price_usd_per_gram": gold_per_gram,
            "status": "alpha",
        }));
    }
    #[cfg(not(feature = "caesar"))]
    {
        Ok(feature_disabled(state, "caesar.balance"))
    }
}

async fn handle_transactions(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    #[cfg(feature = "caesar")]
    {
        let Some(caesar) = state.caesar.as_ref() else {
            let mut v = protocol_unavailable(state, "caesar.transactions");
            v["limit"] = limit.into();
            v["transactions"] = serde_json::json!([]);
            v["count"] = 0.into();
            return Ok(v);
        };
        let protocol = caesar.read().await;
        let settlements = protocol
            .storage()
            .list_recent_settlements(limit)
            .await
            .unwrap_or_default();

        let txs: Vec<serde_json::Value> = settlements
            .iter()
            .map(|s| {
                serde_json::json!({
                    "settlement_id": s.settlement_id,
                    "packet_id": hex::encode(s.packet_id.0),
                    "egress_node": s.egress_node.to_string(),
                    "finality_type": s.finality_type,
                    "fee_collected": s.fee_collected.0.to_f64().unwrap_or(0.0),
                    "settled_at": s.settled_at.to_rfc3339(),
                })
            })
            .collect();

        return Ok(serde_json::json!({
            "node_id": state.node_id,
            "transactions": txs,
            "count": settlements.len(),
            "limit": limit,
            "status": "alpha",
        }));
    }
    #[cfg(not(feature = "caesar"))]
    {
        let mut v = feature_disabled(state, "caesar.transactions");
        v["limit"] = limit.into();
        v["transactions"] = serde_json::json!([]);
        v["count"] = 0.into();
        Ok(v)
    }
}

async fn handle_rewards(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let shard_count = state.shard_store.count().await;

    #[cfg(feature = "caesar")]
    {
        let Some(caesar) = state.caesar.as_ref() else {
            let mut v = protocol_unavailable(state, "caesar.rewards");
            v["shards_hosted"] = shard_count.into();
            return Ok(v);
        };
        let protocol = caesar.read().await;
        let node_id = hypermesh_lib::NodeId::from_public_key(state.node_id.as_bytes());
        let node_status = protocol.storage().get_node_status(&node_id).await.ok().flatten();

        let (lifetime_earned, settled_count) = match &node_status {
            Some(status) => (
                status.total_fees_earned.0.to_f64().unwrap_or(0.0),
                status.settled_count,
            ),
            None => (0.0, 0),
        };

        return Ok(serde_json::json!({
            "node_id": state.node_id,
            "claimable": 0.0,
            "lifetime_earned": lifetime_earned,
            "settled_count": settled_count,
            "shard_serving_rewards": 0.0,
            "dns_hosting_rewards": 0.0,
            "validation_rewards": 0.0,
            "shards_hosted": shard_count,
            "privacy_mode": state.privacy_mode,
            "status": "alpha",
        }));
    }
    #[cfg(not(feature = "caesar"))]
    {
        let mut v = feature_disabled(state, "caesar.rewards");
        v["shards_hosted"] = shard_count.into();
        v["privacy_mode"] = state.privacy_mode.clone().into();
        Ok(v)
    }
}

async fn handle_staking(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    #[cfg(feature = "caesar")]
    {
        if state.caesar.is_none() {
            return Ok(protocol_unavailable(state, "caesar.staking"));
        }
        return Ok(serde_json::json!({
            "node_id": state.node_id,
            "staked": 0.0,
            "lock_period_days": 0,
            "annual_yield_estimate": 0.0,
            "stake_entries": [],
            "status": "alpha",
            "note": "Caesar EVP has no staking primitive; returns honest zeros",
        }));
    }
    #[cfg(not(feature = "caesar"))]
    {
        let mut v = feature_disabled(state, "caesar.staking");
        v["stake_entries"] = serde_json::json!([]);
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::RpcRequest;

    #[tokio::test]
    async fn test_caesar_overview_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("caesar.overview", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert!(result["chain_height"].is_number());
        assert!(result["uptime_secs"].is_number());
    }

    #[tokio::test]
    async fn test_caesar_balance_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("caesar.balance", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["node_id"], "test-node");
    }

    #[tokio::test]
    async fn test_caesar_transactions_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "caesar.transactions",
            serde_json::json!({"limit": 10}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["limit"], 10);
    }

    #[tokio::test]
    async fn test_caesar_rewards_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("caesar.rewards", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert!(result["shards_hosted"].is_number());
    }

    #[tokio::test]
    async fn test_caesar_staking_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("caesar.staking", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    }
}
