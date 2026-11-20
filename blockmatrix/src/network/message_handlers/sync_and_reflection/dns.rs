use super::*;

pub(in crate::network::message_handlers) async fn handle_dns_resolve_request(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    if data.len() < 2 {
        debug!("DNS resolve request too short from {}", peer_node_id);
        return;
    }
    let name = match std::str::from_utf8(&data[1..]) {
        Ok(n) => n,
        Err(_) => {
            debug!("DNS resolve request invalid UTF-8 from {}", peer_node_id);
            return;
        }
    };

    debug!(
        name = name,
        peer = peer_node_id,
        "DNS network resolve request"
    );

    // Check local DNS resolver.
    let response = if let Some(ref dns) = ctx.dns_resolver {
        resolve_dns_response(dns, name).await
    } else {
        vec![TAG_DNS_RESOLVE_RESPONSE]
    };

    if let Err(e) = stream.send(&response).await {
        debug!(
            "Failed to send DNS resolve response to {}: {}",
            peer_node_id, e
        );
    }
}

pub(in crate::network::message_handlers) async fn handle_dns_query(
    payload: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
) {
    let short = &peer_node_id[..8.min(peer_node_id.len())];

    let query: DistributedDnsQuery = match serde_json::from_slice(payload) {
        Ok(q) => q,
        Err(e) => {
            debug!("Invalid DNS query JSON from {}: {}", short, e);
            return;
        }
    };

    let response = build_dns_response_for_query(&query, ctx).await;

    let json = match serde_json::to_vec(&response) {
        Ok(b) => b,
        Err(e) => {
            debug!("Failed to serialize DNS response: {}", e);
            return;
        }
    };

    let mut frame = Vec::with_capacity(1 + json.len());
    frame.push(TAG_DNS_RESPONSE);
    frame.extend_from_slice(&json);

    if let Err(e) = stream.send(&frame).await {
        debug!("Failed to send DNS response to {}: {}", short, e);
    }
}

async fn build_dns_response_for_query(
    query: &DistributedDnsQuery,
    ctx: &PeerContext,
) -> DistributedDnsResponse {
    let chain = ctx.blockchain.get_chain().await;
    let mut accumulated = DnsRecords {
        records: Vec::new(),
        earliest_ts: None,
        earliest_height: 0,
        foundation_grant_present: false,
    };
    for block in chain.iter() {
        accumulated.observe_block(block, &query.domain_name);
    }
    let chain_id = blake3::hash(ctx.network_id.as_bytes()).to_hex().to_string();
    DistributedDnsResponse {
        query_id: query.query_id,
        domain_name: query.domain_name.clone(),
        records: accumulated.records,
        chain_id,
        chain_height: accumulated.earliest_height,
        registration_timestamp: accumulated.earliest_ts.unwrap_or(0),
        foundation_grant_present: accumulated.foundation_grant_present,
    }
}

struct DnsRecords {
    records: Vec<crate::dns::DnsRecord>,
    earliest_ts: Option<u64>,
    earliest_height: u64,
    foundation_grant_present: bool,
}
impl DnsRecords {
    fn observe_block(&mut self, block: &crate::blockchain::block::Block, domain_name: &str) {
        for entry in &block.entries {
            let Some(dns_entry) = matching_dns_entry(entry, domain_name) else {
                continue;
            };
            self.observe_entry(block, entry, dns_entry);
        }
    }

    fn observe_entry(
        &mut self,
        block: &crate::blockchain::block::Block,
        entry: &crate::blockchain::block::BlockAssetEntry,
        dns_entry: crate::dns::DnsBlockEntry,
    ) {
        if dns_entry.grant_signature.is_some() {
            self.foundation_grant_present = true;
        }
        let rec = crate::dns::DnsRecord {
            domain: dns_entry.domain_name.clone(),
            record_type: dns_entry.record_type.clone(),
            data: dns_entry.record_data.clone(),
            ttl: dns_entry.ttl,
            created_at: entry.state_proof.time_proof.time_verification_timestamp,
            expires_at: entry.state_proof.time_proof.time_verification_timestamp,
            owner: dns_entry.owner.clone(),
            tx_hash: Some(block.hash.clone()),
        };
        self.records.push(rec);
        let ts = entry
            .state_proof
            .time_proof
            .time_verification_timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        match self.earliest_ts {
            None => {
                self.earliest_ts = Some(ts);
                self.earliest_height = block.index;
            }
            Some(prev) if ts < prev => {
                self.earliest_ts = Some(ts);
                self.earliest_height = block.index;
            }
            _ => {}
        }
    }
}

fn matching_dns_entry(
    entry: &crate::blockchain::block::BlockAssetEntry,
    domain_name: &str,
) -> Option<crate::dns::DnsBlockEntry> {
    use crate::assets::core::{AssetCategory, BaseSystemType};
    use crate::blockchain::block::StoragePointer;
    let is_dns = matches!(
        entry.registration.category,
        AssetCategory::BaseSystem(BaseSystemType::Dns)
    );
    if !is_dns {
        return None;
    }
    let dns_json = match &entry.storage_pointer {
        StoragePointer::Local { path } => path.as_str(),
        _ => return None,
    };
    let dns_entry: crate::dns::DnsBlockEntry = match serde_json::from_str(dns_json) {
        Ok(e) => e,
        Err(_) => return None,
    };
    if dns_entry.domain_name != domain_name {
        return None;
    }
    Some(dns_entry)
}

async fn resolve_dns_response(dns: &crate::bootstrap::DnsResolver, name: &str) -> Vec<u8> {
    match dns.resolve(name).await {
        Some(addr) => {
            let addr_str = addr.to_string();
            let mut resp = vec![TAG_DNS_RESOLVE_RESPONSE];
            resp.extend_from_slice(addr_str.as_bytes());
            resp
        }
        None => vec![TAG_DNS_RESOLVE_RESPONSE],
    }
}
