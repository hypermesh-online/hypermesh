// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * Server-Sent Events (SSE) client backed by fetch + ReadableStream.
 *
 * Why not `EventSource`?
 * - The browser's native `EventSource` cannot attach custom request headers,
 *   which we need for the `X-HyperMesh-Capability` capability token.
 *
 * This module implements the SSE wire format (`data: ...\n\n`, `event: ...`,
 * comment lines starting with `:`) directly on top of fetch streams so we can
 * carry arbitrary headers uniformly with the rest of the HyperMesh API.
 *
 * Frame shape is type-parameterised; callers pass the expected payload type.
 */

import { useEffect, useRef, useState } from 'react';
import { getConfig } from './config';

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type SseHandler<T> = (event: T) => void;
export type SseErrorHandler = (err: Error) => void;

export interface SseOptions<T> {
  /** Path (relative to the configured API base URL) or absolute URL. */
  url: string;
  /** Extra headers (e.g. `X-HyperMesh-Capability`). */
  headers?: Record<string, string>;
  /** Called for every successfully decoded event. */
  onMessage: SseHandler<T>;
  /** Called on network / parse errors. Reconnect logic continues unless `reconnect === false`. */
  onError?: SseErrorHandler;
  /** Called once when the stream first delivers data (i.e. is considered open). */
  onOpen?: () => void;
  /** Called when the stream closes (intentionally or due to error). */
  onClose?: () => void;
  /** Auto-reconnect on transport failure. Default `true`. */
  reconnect?: boolean;
  /** Base reconnect delay in milliseconds (exponential backoff). Default `1000`. */
  reconnectDelayMs?: number;
  /** Maximum reconnect delay in milliseconds. Default `30_000`. */
  reconnectMaxMs?: number;
}

/**
 * Disposer returned by `openSseStream` — call to cancel and stop reconnecting.
 */
export type SseCloser = () => void;

// ---------------------------------------------------------------------------
// openSseStream
// ---------------------------------------------------------------------------

/**
 * Open an SSE stream. Returns a cancellation function that, when called,
 * aborts the in-flight request and stops any pending reconnect attempts.
 *
 * The function returns synchronously — the actual connection happens in the
 * background.
 */
export function openSseStream<T = unknown>(opts: SseOptions<T>): SseCloser {
  let cancelled = false;
  let controller: AbortController | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let attempt = 0;

  const baseDelay = opts.reconnectDelayMs ?? 1000;
  const maxDelay = opts.reconnectMaxMs ?? 30_000;

  const fullUrl = resolveUrl(opts.url);

  const scheduleReconnect = (err: Error) => {
    if (cancelled) {
      return;
    }
    opts.onError?.(err);
    if (opts.reconnect === false) {
      opts.onClose?.();
      return;
    }
    attempt += 1;
    const delay = Math.min(baseDelay * 2 ** Math.min(attempt - 1, 5), maxDelay);
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      void connect();
    }, delay);
  };

  const connect = async (): Promise<void> => {
    if (cancelled) {
      return;
    }
    controller = new AbortController();
    let opened = false;
    try {
      const response = await fetch(fullUrl, {
        method: 'GET',
        headers: {
          Accept: 'text/event-stream',
          'Cache-Control': 'no-cache',
          ...(opts.headers ?? {}),
        },
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new Error(`SSE connection failed: ${response.status} ${response.statusText}`);
      }
      if (!response.body) {
        throw new Error('SSE response has no body');
      }

      // Successful HTTP handshake — reset backoff for future failures.
      attempt = 0;

      const reader = response.body.getReader();
      const decoder = new TextDecoder('utf-8');
      let buffer = '';

      while (!cancelled) {
        const { value, done } = await reader.read();
        if (done) {
          break;
        }
        // First chunk: consider the stream open.
        if (!opened) {
          opened = true;
          opts.onOpen?.();
        }
        buffer += decoder.decode(value, { stream: true });
        buffer = consumeEvents<T>(buffer, opts);
      }
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      if (cancelled || error.name === 'AbortError') {
        return;
      }
      scheduleReconnect(error);
      return;
    }

    // Reader finished cleanly. Close or reconnect depending on options.
    if (cancelled) {
      return;
    }
    if (opts.reconnect === false) {
      opts.onClose?.();
      return;
    }
    // Server closed the stream — treat as a transient failure.
    scheduleReconnect(new Error('SSE stream closed by server'));
  };

  // Kick off the first connection. Fire-and-forget; errors are reported via callbacks.
  void connect();

  return () => {
    cancelled = true;
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (controller) {
      try {
        controller.abort();
      } catch {
        // Ignore — abort can throw if already finished.
      }
    }
    opts.onClose?.();
  };
}

// ---------------------------------------------------------------------------
// Internal: SSE wire-format parser
// ---------------------------------------------------------------------------

/**
 * Drain complete events from the buffer. Returns the remaining buffer.
 * Events are separated by `\n\n` per the SSE spec.
 *
 * Lines starting with `:` are comments (ignored); `data:` lines accumulate the
 * payload; multi-line `data:` payloads are joined with `\n`.
 */
function consumeEvents<T>(buffer: string, opts: SseOptions<T>): string {
  let remaining = buffer;
  // Normalise CRLF to LF for parser simplicity (browsers SHOULD send LF; some
  // proxies still emit CRLF).
  if (remaining.includes('\r\n')) {
    remaining = remaining.replace(/\r\n/g, '\n');
  }
  while (true) {
    const sep = remaining.indexOf('\n\n');
    if (sep === -1) {
      return remaining;
    }
    const raw = remaining.slice(0, sep);
    remaining = remaining.slice(sep + 2);
    const dataLines: string[] = [];
    for (const line of raw.split('\n')) {
      if (line.startsWith(':')) {
        continue; // comment / keep-alive
      }
      if (line.startsWith('data:')) {
        // Per spec: strip a single leading space if present.
        const payload = line.slice(5);
        dataLines.push(payload.startsWith(' ') ? payload.slice(1) : payload);
      }
      // `event:` / `id:` / `retry:` fields are intentionally ignored for now;
      // the M.6b protocol only uses `data:` and comments.
    }
    if (dataLines.length === 0) {
      continue;
    }
    const payload = dataLines.join('\n');
    try {
      const parsed = JSON.parse(payload) as T;
      opts.onMessage(parsed);
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      opts.onError?.(new Error(`malformed event: ${error.message}`));
    }
  }
}

function resolveUrl(input: string): string {
  if (/^https?:\/\//i.test(input)) {
    return input;
  }
  const base = getConfig().api.baseUrl.replace(/\/$/, '');
  const path = input.startsWith('/') ? input : `/${input}`;
  return `${base}${path}`;
}

// ---------------------------------------------------------------------------
// React hook
// ---------------------------------------------------------------------------

export interface EventStreamState<T> {
  /** Most recently received event payload, or `null` before any frame arrives. */
  last: T | null;
  /** Last transport error, cleared once a frame is successfully received. */
  error: Error | null;
  /** True once the stream is open and producing events. */
  connected: boolean;
}

/**
 * React hook: subscribe to an SSE stream while a component is mounted.
 *
 * Pass `null` as `url` to disable the subscription (useful for conditional
 * streams). `headers` should be a stable object; the hook re-subscribes when
 * the JSON-stringified value changes.
 */
export function useEventStream<T = unknown>(
  url: string | null,
  headers?: Record<string, string>,
): EventStreamState<T> {
  const [last, setLast] = useState<T | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [connected, setConnected] = useState(false);

  // Stable JSON snapshot of headers to avoid re-subscribing on every render.
  const headersKey = headers ? JSON.stringify(headers) : '';
  const headersRef = useRef(headers);
  headersRef.current = headers;

  useEffect(() => {
    if (!url) {
      setConnected(false);
      return;
    }
    let active = true;
    setError(null);
    setConnected(false);

    const close = openSseStream<T>({
      url,
      headers: headersRef.current,
      onMessage: (ev) => {
        if (!active) return;
        setLast(ev);
        setError(null);
        if (!connected) {
          setConnected(true);
        }
      },
      onOpen: () => {
        if (!active) return;
        setConnected(true);
      },
      onError: (err) => {
        if (!active) return;
        setError(err);
        setConnected(false);
      },
      onClose: () => {
        if (!active) return;
        setConnected(false);
      },
    });

    return () => {
      active = false;
      close();
    };
    // headersKey intentionally triggers re-subscription on header content change.

  }, [url, headersKey]);

  return { last, error, connected };
}
