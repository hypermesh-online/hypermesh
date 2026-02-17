// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/* tslint:disable */
/* eslint-disable */
export function create_stoq_message(message_type: string, payload: string, correlation_id: string): any;
/**
 * WebAssembly entry point - initialize the STOQ WASM client
 */
export function main(): void;
/**
 * Certificate handling for WebAssembly
 */
export class WasmCertificate {
  free(): void;
  constructor(pem_data: string);
  validate(): boolean;
  readonly fingerprint: string;
}
/**
 * Configuration for WASM STOQ connection
 */
export class WasmConnectionConfig {
  free(): void;
  constructor(server_address: string, server_port: number, use_ipv6: boolean);
  readonly server_address: string;
  readonly server_port: number;
  readonly use_ipv6: boolean;
}
/**
 * Connection status for WASM client - using simple fields
 */
export class WasmConnectionStatus {
  free(): void;
  constructor();
  readonly is_connected: boolean;
  readonly is_authenticated: boolean;
  readonly connection_id: string;
  readonly error_message: string;
  readonly protocol_version: string;
}
/**
 * Main WASM STOQ Client
 */
export class WasmStoqClient {
  free(): void;
  constructor(server_address: string, server_port: number, use_ipv6: boolean);
  /**
   * Initialize connection with certificate
   */
  connect(certificate: WasmCertificate): Promise<void>;
  /**
   * Send a message (using JsValue instead of struct reference)
   */
  send_message(message_js: any): Promise<any>;
  /**
   * Disconnect from server
   */
  disconnect(): Promise<void>;
  readonly status: WasmConnectionStatus;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmconnectionconfig_free: (a: number, b: number) => void;
  readonly wasmconnectionconfig_new: (a: number, b: number, c: number, d: number) => number;
  readonly wasmconnectionconfig_server_address: (a: number) => [number, number];
  readonly wasmconnectionconfig_server_port: (a: number) => number;
  readonly wasmconnectionconfig_use_ipv6: (a: number) => number;
  readonly create_stoq_message: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
  readonly __wbg_wasmcertificate_free: (a: number, b: number) => void;
  readonly wasmcertificate_new: (a: number, b: number) => [number, number, number];
  readonly wasmcertificate_fingerprint: (a: number) => [number, number];
  readonly wasmcertificate_validate: (a: number) => number;
  readonly __wbg_wasmconnectionstatus_free: (a: number, b: number) => void;
  readonly wasmconnectionstatus_new: () => number;
  readonly wasmconnectionstatus_is_connected: (a: number) => number;
  readonly wasmconnectionstatus_is_authenticated: (a: number) => number;
  readonly wasmconnectionstatus_connection_id: (a: number) => [number, number];
  readonly wasmconnectionstatus_error_message: (a: number) => [number, number];
  readonly wasmconnectionstatus_protocol_version: (a: number) => [number, number];
  readonly __wbg_wasmstoqclient_free: (a: number, b: number) => void;
  readonly wasmstoqclient_new: (a: number, b: number, c: number, d: number) => number;
  readonly wasmstoqclient_connect: (a: number, b: number) => any;
  readonly wasmstoqclient_send_message: (a: number, b: any) => any;
  readonly wasmstoqclient_status: (a: number) => number;
  readonly wasmstoqclient_disconnect: (a: number) => any;
  readonly main: () => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_4: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly closure128_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure158_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
