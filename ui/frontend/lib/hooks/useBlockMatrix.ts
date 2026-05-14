// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * React Query hooks for the real BlockMatrix HTTP API.
 *
 * These hooks call the actual endpoints on the Gateway (localhost:8443) and
 * return typed data. They intentionally do NOT fall back to mock
 * data -- if the backend is down the query will be in error state
 * and the UI should show a "not connected" indicator.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  blockMatrixClient,
  type NodeStatus,
  type BlockData,
  type DnsRecord,
  type TopologyInfo,
  type TopologyNeighbor,
  type PeerInfo,
  type AssetRecord,
  type AssetRegisterInput,
  type AssetRegisterResponse,
  type DomainRecord,
  type ShareInboxResponse,
  type ShareActionResponse,
  type MessageInboxResponse,
  type MessageItem,
  type CaesarOverview,
  type CaesarBalance,
  type TransactionList,
  type RewardsInfo,
  type StakingInfo,
  type CapacityMetrics,
  type TrafficAnalysis,
  type ThrottleStatus,
  type RoutingAdvisory,
  type TrustChainStatus,
  type CertList,
  type IdentityInfo,
  type FederationInfo,
  type StoqStats,
  type ConnectionList,
  type PerformanceMetrics,
} from '../blockmatrix-api';

// ---------- Core ----------

/** Node status (node_id, coordinate, chain_height, peers, uptime, privacy) */
export function useNodeStatus(pollInterval = 10_000) {
  return useQuery<NodeStatus>({
    queryKey: ['blockmatrix', 'status'],
    queryFn: () => blockMatrixClient.getStatus(),
    refetchInterval: pollInterval,
    staleTime: 5_000,
    retry: 2,
  });
}

/** Simple liveness check */
export function usePing() {
  return useQuery<string>({
    queryKey: ['blockmatrix', 'ping'],
    queryFn: () => blockMatrixClient.ping(),
    refetchInterval: 30_000,
    staleTime: 15_000,
    retry: 1,
  });
}

// ---------- Blockchain ----------

/** Current chain height */
export function useBlockchainHeight(pollInterval = 10_000) {
  return useQuery<{ height: number }>({
    queryKey: ['blockmatrix', 'blockchain', 'height'],
    queryFn: () => blockMatrixClient.getBlockchainHeight(),
    refetchInterval: pollInterval,
    staleTime: 5_000,
  });
}

/** Fetch a single block by index */
export function useBlock(index: number | undefined) {
  return useQuery<BlockData>({
    queryKey: ['blockmatrix', 'blockchain', 'block', index],
    queryFn: () => blockMatrixClient.getBlock(index!),
    enabled: index !== undefined,
    staleTime: 60_000, // blocks are immutable
  });
}

/** Validate the full chain */
export function useChainValidation() {
  return useQuery<{ valid: boolean; height: number }>({
    queryKey: ['blockmatrix', 'blockchain', 'validate'],
    queryFn: () => blockMatrixClient.validateChain(),
    staleTime: 30_000,
    retry: 1,
  });
}

// ---------- DNS ----------

/** List all DNS records */
export function useDnsList(pollInterval = 15_000) {
  return useQuery<DnsRecord[]>({
    queryKey: ['blockmatrix', 'dns', 'list'],
    queryFn: () => blockMatrixClient.getDnsList(),
    refetchInterval: pollInterval,
    staleTime: 10_000,
  });
}

/** Resolve a single DNS name */
export function useDnsResolve(name: string | undefined) {
  return useQuery<DnsRecord>({
    queryKey: ['blockmatrix', 'dns', 'resolve', name],
    queryFn: () => blockMatrixClient.resolveDns(name!),
    enabled: !!name,
    staleTime: 30_000,
  });
}

/** Register a DNS record (mutation) */
export function useRegisterDns() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: { name: string; address: string }) =>
      blockMatrixClient.registerDns(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'dns'] });
    },
  });
}

// ---------- Network ----------

/** Connected peers */
export function useNetworkPeers(pollInterval = 15_000) {
  return useQuery<PeerInfo[]>({
    queryKey: ['blockmatrix', 'network', 'peers'],
    queryFn: () => blockMatrixClient.getNetworkPeers(),
    refetchInterval: pollInterval,
    staleTime: 10_000,
  });
}

// ---------- Topology ----------

/** This node's matrix position and metadata */
export function useTopologyInfo() {
  return useQuery<TopologyInfo>({
    queryKey: ['blockmatrix', 'topology', 'info'],
    queryFn: () => blockMatrixClient.getTopologyInfo(),
    staleTime: 60_000, // topology doesn't change often
  });
}

/** Neighbors in the matrix */
export function useTopologyNeighbors() {
  return useQuery<TopologyNeighbor[]>({
    queryKey: ['blockmatrix', 'topology', 'neighbors'],
    queryFn: () => blockMatrixClient.getTopologyNeighbors(),
    staleTime: 30_000,
  });
}

// ---------- Assets ----------

/** List all blockchain-registered assets */
export function useAssetList(pollInterval = 15_000) {
  return useQuery<AssetRecord[]>({
    queryKey: ['blockmatrix', 'asset', 'list'],
    queryFn: () => blockMatrixClient.getAssetList(),
    refetchInterval: pollInterval,
    staleTime: 10_000,
  });
}

/** Register a new asset on the blockchain (mutation) */
export function useRegisterAsset() {
  const qc = useQueryClient();
  return useMutation<AssetRegisterResponse, Error, AssetRegisterInput>({
    mutationFn: (input) => blockMatrixClient.registerAsset(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'asset', 'list'] });
    },
  });
}

// ---------- Config ----------

/** Full node configuration */
export function useConfigShow() {
  return useQuery<Record<string, unknown>>({
    queryKey: ['blockmatrix', 'config', 'show'],
    queryFn: () => blockMatrixClient.getConfig() as Promise<Record<string, unknown>>,
    staleTime: 30_000,
  });
}

/** Get a specific config key */
export function useConfigGet(key: string | undefined) {
  return useQuery<unknown>({
    queryKey: ['blockmatrix', 'config', 'get', key],
    queryFn: () => blockMatrixClient.getConfigKey(key!),
    enabled: !!key,
    staleTime: 30_000,
  });
}

// ---------- Domain ----------

/** List registered domains */
export function useDomainList(pollInterval = 30_000) {
  return useQuery<DomainRecord[]>({
    queryKey: ['blockmatrix', 'domain', 'list'],
    queryFn: () => blockMatrixClient.getDomainList(),
    refetchInterval: pollInterval,
    staleTime: 15_000,
  });
}

/** Register a new domain (mutation) */
export function useRegisterDomain() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: { name: string }) =>
      blockMatrixClient.registerDomain(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'domain'] });
    },
  });
}

/** Join an existing domain (mutation) */
export function useJoinDomain() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: { domain: string; token: string }) =>
      blockMatrixClient.joinDomain(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'domain'] });
    },
  });
}

// ---------- Sharing ----------

/** Poll the share inbox for received invites */
export function useShareInbox(pollInterval = 10_000) {
  return useQuery<ShareInboxResponse>({
    queryKey: ['blockmatrix', 'share', 'inbox'],
    queryFn: () => blockMatrixClient.shareInbox(),
    refetchInterval: pollInterval,
    staleTime: 5_000,
    retry: 1,
  });
}

/** Send a share invite to a recipient */
export function useShareSend() {
  const qc = useQueryClient();
  return useMutation<ShareActionResponse, Error, { assetId: string; recipient: string }>({
    mutationFn: ({ assetId, recipient }) =>
      blockMatrixClient.shareSend(assetId, recipient),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'share'] });
    },
  });
}

/** Accept a share invite */
export function useShareAccept() {
  const qc = useQueryClient();
  return useMutation<ShareActionResponse, Error, string>({
    mutationFn: (inviteId) => blockMatrixClient.shareAccept(inviteId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'share', 'inbox'] });
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'asset'] });
    },
  });
}

/** Reject a share invite */
export function useShareReject() {
  const qc = useQueryClient();
  return useMutation<ShareActionResponse, Error, string>({
    mutationFn: (inviteId) => blockMatrixClient.shareReject(inviteId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'share', 'inbox'] });
    },
  });
}

// ---------- Messaging ----------

/** Poll the message inbox */
export function useMessageInbox(pollInterval = 10_000) {
  return useQuery<MessageInboxResponse>({
    queryKey: ['blockmatrix', 'message', 'inbox'],
    queryFn: () => blockMatrixClient.messageInbox(),
    refetchInterval: pollInterval,
    staleTime: 5_000,
    retry: 1,
  });
}

/** Fetch message history with a specific peer */
export function useMessageHistory(peer: string, limit = 50) {
  return useQuery<MessageInboxResponse>({
    queryKey: ['blockmatrix', 'message', 'history', peer],
    queryFn: () => blockMatrixClient.messageHistory(peer, limit),
    enabled: !!peer,
    refetchInterval: 5_000,
    staleTime: 3_000,
  });
}

/** Send a message (mutation) */
export function useMessageSend() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ recipient, body, contentType, replyTo }: {
      recipient: string;
      body: string;
      contentType?: string;
      replyTo?: string;
    }) => blockMatrixClient.messageSend(recipient, body, contentType, replyTo),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'message'] });
    },
  });
}

/** Mark a message as read (mutation) */
export function useMessageRead() {
  const qc = useQueryClient();
  return useMutation<{ message: MessageItem }, Error, string>({
    mutationFn: (messageId) => blockMatrixClient.messageRead(messageId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['blockmatrix', 'message', 'inbox'] });
    },
  });
}

// ---------- Caesar ----------

/** Caesar overview (balance, staking, recent tx count) */
export function useCaesarOverview(pollInterval = 30_000) {
  return useQuery<CaesarOverview>({
    queryKey: ['caesar', 'overview'],
    queryFn: () => blockMatrixClient.caesarOverview(),
    refetchInterval: pollInterval,
    staleTime: 15_000,
    retry: 2,
  });
}

/** Caesar balance */
export function useCaesarBalance(pollInterval = 10_000) {
  return useQuery<CaesarBalance>({
    queryKey: ['caesar', 'balance'],
    queryFn: () => blockMatrixClient.caesarBalance(),
    refetchInterval: pollInterval,
    staleTime: 5_000,
  });
}

/** Caesar transaction list */
export function useCaesarTransactions(limit?: number) {
  return useQuery<TransactionList>({
    queryKey: ['caesar', 'transactions', limit],
    queryFn: () => blockMatrixClient.caesarTransactions(limit),
    staleTime: 10_000,
  });
}

/** Caesar rewards info */
export function useCaesarRewards(pollInterval = 30_000) {
  return useQuery<RewardsInfo>({
    queryKey: ['caesar', 'rewards'],
    queryFn: () => blockMatrixClient.caesarRewards(),
    refetchInterval: pollInterval,
    staleTime: 15_000,
  });
}

/** Caesar staking info */
export function useCaesarStaking(pollInterval = 60_000) {
  return useQuery<StakingInfo>({
    queryKey: ['caesar', 'staking'],
    queryFn: () => blockMatrixClient.caesarStaking(),
    refetchInterval: pollInterval,
    staleTime: 30_000,
  });
}

// ---------- Engauge ----------

/** Engauge capacity metrics */
export function useEngaugeCapacity(pollInterval = 5_000) {
  return useQuery<CapacityMetrics>({
    queryKey: ['engauge', 'capacity'],
    queryFn: () => blockMatrixClient.engaugeCapacity(),
    refetchInterval: pollInterval,
    staleTime: 3_000,
  });
}

/** Engauge traffic analysis */
export function useEngaugeTraffic(pollInterval = 10_000) {
  return useQuery<TrafficAnalysis>({
    queryKey: ['engauge', 'traffic'],
    queryFn: () => blockMatrixClient.engaugeTraffic(),
    refetchInterval: pollInterval,
    staleTime: 5_000,
  });
}

/** Engauge throttle status */
export function useEngaugeThrottle(pollInterval = 5_000) {
  return useQuery<ThrottleStatus>({
    queryKey: ['engauge', 'throttle'],
    queryFn: () => blockMatrixClient.engaugeThrottle(),
    refetchInterval: pollInterval,
    staleTime: 3_000,
  });
}

/** Engauge routing advisory */
export function useEngaugeRouting(pollInterval = 15_000) {
  return useQuery<RoutingAdvisory>({
    queryKey: ['engauge', 'routing'],
    queryFn: () => blockMatrixClient.engaugeRouting(),
    refetchInterval: pollInterval,
    staleTime: 10_000,
  });
}

// ---------- TrustChain ----------

/** TrustChain CA status */
export function useTrustchainStatus(pollInterval = 30_000) {
  return useQuery<TrustChainStatus>({
    queryKey: ['trustchain', 'status'],
    queryFn: () => blockMatrixClient.trustchainStatus(),
    refetchInterval: pollInterval,
    staleTime: 15_000,
  });
}

/** TrustChain certificate list */
export function useTrustchainCerts(pollInterval = 60_000) {
  return useQuery<CertList>({
    queryKey: ['trustchain', 'certs'],
    queryFn: () => blockMatrixClient.trustchainCerts(),
    refetchInterval: pollInterval,
    staleTime: 30_000,
  });
}

/** TrustChain identity info (static, no polling) */
export function useTrustchainIdentity() {
  return useQuery<IdentityInfo>({
    queryKey: ['trustchain', 'identity'],
    queryFn: () => blockMatrixClient.trustchainIdentity(),
    staleTime: 300_000,
  });
}

/** TrustChain federation info */
export function useTrustchainFederation(pollInterval = 30_000) {
  return useQuery<FederationInfo>({
    queryKey: ['trustchain', 'federation'],
    queryFn: () => blockMatrixClient.trustchainFederation(),
    refetchInterval: pollInterval,
    staleTime: 15_000,
  });
}

// ---------- STOQ ----------

/** STOQ transport stats */
export function useStoqStats(pollInterval = 5_000) {
  return useQuery<StoqStats>({
    queryKey: ['stoq', 'stats'],
    queryFn: () => blockMatrixClient.stoqStats(),
    refetchInterval: pollInterval,
    staleTime: 3_000,
  });
}

/** STOQ active connections */
export function useStoqConnections(pollInterval = 10_000) {
  return useQuery<ConnectionList>({
    queryKey: ['stoq', 'connections'],
    queryFn: () => blockMatrixClient.stoqConnections(),
    refetchInterval: pollInterval,
    staleTime: 5_000,
  });
}

/** STOQ performance metrics */
export function useStoqPerformance(pollInterval = 5_000) {
  return useQuery<PerformanceMetrics>({
    queryKey: ['stoq', 'performance'],
    queryFn: () => blockMatrixClient.stoqPerformance(),
    refetchInterval: pollInterval,
    staleTime: 3_000,
  });
}
