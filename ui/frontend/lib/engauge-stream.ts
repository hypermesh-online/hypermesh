// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * Engauge SSE streaming types and helpers (Phase M.6b).
 *
 * Mirrors the wire-format defined in `engauge/src/streaming/protocol.rs`.
 * Frames are received over `GET /api/v1/blockmatrix/engauge/stream` (and the
 * alias `/api/v1/engauge/stream`) as `text/event-stream` `data:` payloads.
 */

import { useEventStream, type EventStreamState } from './sse';

// ---------------------------------------------------------------------------
// Wire types — keep in sync with engauge protocol.rs
// ---------------------------------------------------------------------------

export type PrivacyMode =
  | { scope: 'Unbounded'; tracked: false } // ANONYMOUS
  | { scope: 'Bounded'; tracked: true } // PRIVATE
  | { scope: 'Unbounded'; tracked: true } // PUBLIC
  | { scope: string; tracked: boolean }; // fallback for forward-compat

export interface CapacitySnapshot {
  bytes_served: number;
  compute_delivered: number;
  storage_maintained_bytes: number;
  bandwidth_available_bps: number;
  uptime_ratio: number;
}

export interface CongestionSnapshot {
  buffer_fullness_ratio: number;
  queue_depth: number;
  dropped_packets_epoch: number;
  avg_queue_wait_us: number;
}

export interface RoutingSnapshot {
  avg_latency_us: number;
  throughput_bps: number;
  path_count: number;
  active_connections: number;
}

export interface EconomicSnapshot {
  in_flight_float_grams: number;
  settlement_rate_per_epoch: number;
  active_packets: number;
  holdings_by_tier_grams: [number, number, number, number];
  fee_rate_per_epoch_grams: number;
  in_transit_count: number;
  in_transit_value_grams: number;
}

export interface VerificationSnapshot {
  probes_sent: number;
  probes_passed: number;
  avg_response_time_us: number;
  consistency_ratio: number;
  epoch: number;
}

/**
 * Tagged union mirroring the Rust `MetricsPayload` enum. Serde uses
 * externally-tagged JSON (variant name as the key), so the TS shape is
 * `{ Capacity: CapacitySnapshot } | { Congestion: ... } | ...`.
 */
export type MetricsPayload =
  | { Capacity: CapacitySnapshot }
  | { Congestion: CongestionSnapshot }
  | { Routing: RoutingSnapshot }
  | { Economic: EconomicSnapshot }
  | { Verification: VerificationSnapshot };

export interface MetricsFrame {
  source_node: string; // NodeId is serialised as a hex string
  timestamp_us: number;
  privacy_mode: PrivacyMode;
  payload: MetricsPayload;
  sequence: number;
}

// ---------------------------------------------------------------------------
// Payload helpers
// ---------------------------------------------------------------------------

export type FrameKind = 'Capacity' | 'Congestion' | 'Routing' | 'Economic' | 'Verification';

export function frameKind(frame: MetricsFrame): FrameKind | null {
  const keys = Object.keys(frame.payload) as FrameKind[];
  return keys.length === 1 ? keys[0] : null;
}

export function capacityOf(frame: MetricsFrame): CapacitySnapshot | null {
  return 'Capacity' in frame.payload ? frame.payload.Capacity : null;
}

export function congestionOf(frame: MetricsFrame): CongestionSnapshot | null {
  return 'Congestion' in frame.payload ? frame.payload.Congestion : null;
}

export function routingOf(frame: MetricsFrame): RoutingSnapshot | null {
  return 'Routing' in frame.payload ? frame.payload.Routing : null;
}

export function economicOf(frame: MetricsFrame): EconomicSnapshot | null {
  return 'Economic' in frame.payload ? frame.payload.Economic : null;
}

export function verificationOf(frame: MetricsFrame): VerificationSnapshot | null {
  return 'Verification' in frame.payload ? frame.payload.Verification : null;
}

// ---------------------------------------------------------------------------
// Convenience hook
// ---------------------------------------------------------------------------

export const ENGAUGE_SSE_PATH = '/api/v1/blockmatrix/engauge/stream';

export interface UseEngaugeStreamOptions {
  /** Capability token forwarded as `X-HyperMesh-Capability`. Omit in alpha. */
  capability?: string;
  /** Pass `false` to disable the subscription. Default `true`. */
  enabled?: boolean;
  /** Override the SSE path. Default `/api/v1/blockmatrix/engauge/stream`. */
  path?: string;
}

/**
 * Subscribe to the engauge SSE metrics stream.
 *
 * Returns the most recent frame, current connection state, and last error.
 * The connection is automatically established on mount and torn down on
 * unmount; reconnect-with-backoff is handled by `openSseStream` internally.
 */
export function useEngaugeStream(
  opts: UseEngaugeStreamOptions = {},
): EventStreamState<MetricsFrame> {
  const { capability, enabled = true, path = ENGAUGE_SSE_PATH } = opts;
  const headers: Record<string, string> | undefined = capability
    ? { 'X-HyperMesh-Capability': capability }
    : undefined;
  return useEventStream<MetricsFrame>(enabled ? path : null, headers);
}
