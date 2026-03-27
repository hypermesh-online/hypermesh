// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Caesar EVP IPC handlers: wallet overview, balance, transactions, rewards, staking.
//!
//! Alpha: The blockmatrix daemon does not yet run a full Caesar protocol. These
//! handlers return structured JSON with correct field names but zero/empty values
//! for Caesar-specific data (EVP packets, gold oracle). Data that *is* available
//! from [`DaemonState`] (chain height, node ID, uptime) is returned live.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::state::DaemonState;

/// Register Caesar-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // caesar.overview -- wallet summary
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

    // caesar.balance -- balance breakdown
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

    // caesar.transactions -- recent transactions
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

    // caesar.rewards -- claimable rewards
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

    // caesar.staking -- staking info
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

/// Wallet summary: balance snapshot, active packets, chain height, status.
async fn handle_overview(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let chain_height = state.blockchain.get_height().await;
    let uptime = state.started_at.elapsed().as_secs();

    Ok(serde_json::json!({
        "balance": {
            "total": 0.0,
            "available": 0.0,
            "locked": 0.0,
            "pending": 0.0,
            "staked": 0.0,
        },
        "active_packets": 0,
        "pending_rewards": 0.0,
        "chain_height": chain_height,
        "node_id": state.node_id,
        "uptime_secs": uptime,
        "status": "alpha",
    }))
}

/// Detailed balance breakdown: total, available, locked, pending, staked.
async fn handle_balance(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    Ok(serde_json::json!({
        "node_id": state.node_id,
        "total": 0.0,
        "available": 0.0,
        "locked": 0.0,
        "pending": 0.0,
        "staked": 0.0,
        "currency": "CAES",
        "gold_backing_grams": 0.0,
        "status": "alpha",
    }))
}

/// Recent transactions list. Accepts optional `limit` param (default 50).
async fn handle_transactions(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50);

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "transactions": [],
        "count": 0,
        "limit": limit,
        "status": "alpha",
        "note": "Caesar protocol not yet running in daemon",
    }))
}

/// Claimable rewards from network participation, DNS hosting, shard serving.
async fn handle_rewards(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let shard_count = state.shard_store.count().await;

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "claimable": 0.0,
        "lifetime_earned": 0.0,
        "shard_serving_rewards": 0.0,
        "dns_hosting_rewards": 0.0,
        "validation_rewards": 0.0,
        "shards_hosted": shard_count,
        "privacy_mode": state.privacy_mode,
        "status": "alpha",
    }))
}

/// Staking overview: amount staked, lock period, annual yield estimate.
async fn handle_staking(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    Ok(serde_json::json!({
        "node_id": state.node_id,
        "staked": 0.0,
        "lock_period_days": 0,
        "annual_yield_estimate": 0.0,
        "stake_entries": [],
        "status": "alpha",
    }))
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

        assert_eq!(result["status"], "alpha");
        assert!(result["chain_height"].is_number());
        assert!(result["balance"]["total"].is_number());
        assert!(result["balance"]["available"].is_number());
        assert!(result["balance"]["locked"].is_number());
        assert!(result["balance"]["pending"].is_number());
        assert!(result["balance"]["staked"].is_number());
        assert_eq!(result["active_packets"], 0);
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

        assert_eq!(result["currency"], "CAES");
        assert_eq!(result["total"], 0.0);
        assert_eq!(result["available"], 0.0);
        assert_eq!(result["gold_backing_grams"], 0.0);
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

        assert_eq!(result["count"], 0);
        assert_eq!(result["limit"], 10);
        assert!(result["transactions"].is_array());
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

        assert_eq!(result["claimable"], 0.0);
        assert_eq!(result["shards_hosted"], 0);
        assert!(result["privacy_mode"].is_string());
    }

    #[tokio::test]
    async fn test_caesar_staking_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("caesar.staking", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["staked"], 0.0);
        assert!(result["stake_entries"].is_array());
    }
}
