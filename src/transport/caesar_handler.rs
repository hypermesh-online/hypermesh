//! Caesar API Handler for STOQ Transport Layer
//!
//! Bridges Caesar economic API endpoints with the STOQ transport layer,
//! allowing the UI to fetch real economic data over the Internet 2.0 protocol.

use anyhow::Result;
use std::sync::Arc;
use serde_json;
use tracing::{info, debug, error};
use uuid;

use crate::transport::http_gateway::{RouteHandler, HttpRequest, HttpResponse};
use caesar::CaesarEconomicSystem;

/// Caesar API handler for economic endpoints
pub struct CaesarApiHandler {
    caesar: Arc<CaesarEconomicSystem>,
}

impl CaesarApiHandler {
    /// Create new Caesar API handler
    pub fn new(caesar: Arc<CaesarEconomicSystem>) -> Self {
        Self { caesar }
    }

    /// Handle Caesar API requests
    async fn handle_caesar_request(&self, path: &str, method: &str, body: &[u8]) -> Result<Vec<u8>> {
        debug!("Caesar API request: {} {}", method, path);

        // Parse path to determine which endpoint
        let response = match (method, path) {
            // Wallet endpoints
            ("GET", "/api/v1/caesar/wallet") => {
                // Parse query params from body (simplified for now)
                let wallet_id = "DEFAULT_WALLET"; // In production, extract from request

                match self.caesar.get_wallet_info(wallet_id).await {
                    Ok(wallet) => serde_json::to_vec(&wallet)?,
                    Err(e) => {
                        error!("Failed to get wallet: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // Balance endpoint
            ("GET", path) if path.starts_with("/api/v1/caesar/wallet/") && path.ends_with("/balance") => {
                let wallet_id = path
                    .strip_prefix("/api/v1/caesar/wallet/")
                    .and_then(|s| s.strip_suffix("/balance"))
                    .unwrap_or("DEFAULT_WALLET");

                match self.caesar.get_wallet_balance(wallet_id).await {
                    Ok(balance) => serde_json::to_vec(&balance)?,
                    Err(e) => {
                        error!("Failed to get balance: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // Rewards endpoint
            ("GET", "/api/v1/caesar/rewards") => {
                let wallet_id = "DEFAULT_WALLET"; // Extract from request in production

                match self.caesar.get_rewards_info(wallet_id).await {
                    Ok(rewards) => serde_json::to_vec(&rewards)?,
                    Err(e) => {
                        error!("Failed to get rewards: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // Staking info endpoint
            ("GET", "/api/v1/caesar/staking") => {
                let wallet_id = "DEFAULT_WALLET"; // Extract from request in production

                match self.caesar.get_staking_details(wallet_id).await {
                    Ok(staking) => serde_json::to_vec(&staking)?,
                    Err(e) => {
                        error!("Failed to get staking info: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // Exchange rates endpoint
            ("GET", "/api/v1/caesar/exchange/rates") => {
                match self.caesar.get_current_exchange_rates().await {
                    Ok(rates) => serde_json::to_vec(&rates)?,
                    Err(e) => {
                        error!("Failed to get exchange rates: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // Transactions endpoint
            ("GET", "/api/v1/caesar/transactions") => {
                let wallet_id = "DEFAULT_WALLET"; // Extract from request in production

                match self.caesar.get_wallet_transactions(wallet_id).await {
                    Ok(transactions) => serde_json::to_vec(&transactions)?,
                    Err(e) => {
                        error!("Failed to get transactions: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // Analytics overview
            ("GET", "/api/v1/caesar/analytics/overview") => {
                let wallet_id = Some("DEFAULT_WALLET".to_string());

                match self.caesar.get_analytics_data(wallet_id.as_ref()).await {
                    Ok(analytics) => serde_json::to_vec(&analytics)?,
                    Err(e) => {
                        error!("Failed to get analytics: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // Earnings breakdown
            ("GET", "/api/v1/caesar/analytics/earnings") => {
                let wallet_id = "DEFAULT_WALLET"; // Extract from request in production

                match self.caesar.get_earnings_details(wallet_id).await {
                    Ok(earnings) => serde_json::to_vec(&earnings)?,
                    Err(e) => {
                        error!("Failed to get earnings breakdown: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": e.to_string()
                        }))?
                    }
                }
            },

            // POST endpoints for transactions
            ("POST", "/api/v1/caesar/transactions/send") => {
                // Parse request body
                match serde_json::from_slice(body) {
                    Ok(request) => {
                        match self.caesar.process_transaction(request).await {
                            Ok(response) => serde_json::to_vec(&response)?,
                            Err(e) => {
                                error!("Transaction failed: {}", e);
                                serde_json::to_vec(&serde_json::json!({
                                    "error": e.to_string()
                                }))?
                            }
                        }
                    }
                    Err(e) => {
                        error!("Invalid request body: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": "Invalid request body"
                        }))?
                    }
                }
            },

            // POST endpoint for claiming rewards
            ("POST", "/api/v1/caesar/rewards/claim") => {
                match serde_json::from_slice(body) {
                    Ok(request) => {
                        match self.caesar.claim_pending_rewards(request).await {
                            Ok(response) => serde_json::to_vec(&response)?,
                            Err(e) => {
                                error!("Failed to claim rewards: {}", e);
                                serde_json::to_vec(&serde_json::json!({
                                    "error": e.to_string()
                                }))?
                            }
                        }
                    }
                    Err(e) => {
                        error!("Invalid request body: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": "Invalid request body"
                        }))?
                    }
                }
            },

            // POST endpoint for staking
            ("POST", "/api/v1/caesar/staking/stake") => {
                match serde_json::from_slice(body) {
                    Ok(request) => {
                        match self.caesar.stake_tokens_for_wallet(request).await {
                            Ok(response) => serde_json::to_vec(&response)?,
                            Err(e) => {
                                error!("Failed to stake tokens: {}", e);
                                serde_json::to_vec(&serde_json::json!({
                                    "error": e.to_string()
                                }))?
                            }
                        }
                    }
                    Err(e) => {
                        error!("Invalid request body: {}", e);
                        serde_json::to_vec(&serde_json::json!({
                            "error": "Invalid request body"
                        }))?
                    }
                }
            },

            // Default 404
            _ => {
                debug!("Caesar API endpoint not found: {} {}", method, path);
                serde_json::to_vec(&serde_json::json!({
                    "error": "Endpoint not found"
                }))?
            }
        };

        Ok(response)
    }
}

/// Route handler implementation for Caesar API
pub struct CaesarRouteHandler {
    caesar: Arc<CaesarEconomicSystem>,
}

impl CaesarRouteHandler {
    pub fn new(caesar: Arc<CaesarEconomicSystem>) -> Self {
        Self { caesar }
    }
}

impl RouteHandler for CaesarRouteHandler {
    fn handle(&self, request: &HttpRequest) -> Result<HttpResponse> {
        // Block on async operation (not ideal but necessary for trait compatibility)
        let caesar = self.caesar.clone();
        let path = request.path.clone();
        let method = request.method.clone();
        let body = request.body.clone();

        let response_body = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = CaesarApiHandler::new(caesar);
                handler.handle_caesar_request(&path, &method, &body).await
            })
        })?;

        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());

        Ok(HttpResponse {
            status: 200,
            headers,
            body: response_body,
        })
    }
}

