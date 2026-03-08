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
  type DomainRecord,
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
