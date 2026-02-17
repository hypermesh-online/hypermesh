// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const __wbg_wasmconnectionconfig_free: (a: number, b: number) => void;
export const wasmconnectionconfig_new: (a: number, b: number, c: number, d: number) => number;
export const wasmconnectionconfig_server_address: (a: number) => [number, number];
export const wasmconnectionconfig_server_port: (a: number) => number;
export const wasmconnectionconfig_use_ipv6: (a: number) => number;
export const create_stoq_message: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
export const __wbg_wasmcertificate_free: (a: number, b: number) => void;
export const wasmcertificate_new: (a: number, b: number) => [number, number, number];
export const wasmcertificate_fingerprint: (a: number) => [number, number];
export const wasmcertificate_validate: (a: number) => number;
export const __wbg_wasmconnectionstatus_free: (a: number, b: number) => void;
export const wasmconnectionstatus_new: () => number;
export const wasmconnectionstatus_is_connected: (a: number) => number;
export const wasmconnectionstatus_is_authenticated: (a: number) => number;
export const wasmconnectionstatus_connection_id: (a: number) => [number, number];
export const wasmconnectionstatus_error_message: (a: number) => [number, number];
export const wasmconnectionstatus_protocol_version: (a: number) => [number, number];
export const __wbg_wasmstoqclient_free: (a: number, b: number) => void;
export const wasmstoqclient_new: (a: number, b: number, c: number, d: number) => number;
export const wasmstoqclient_connect: (a: number, b: number) => any;
export const wasmstoqclient_send_message: (a: number, b: any) => any;
export const wasmstoqclient_status: (a: number) => number;
export const wasmstoqclient_disconnect: (a: number) => any;
export const main: () => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_exn_store: (a: number) => void;
export const __externref_table_alloc: () => number;
export const __wbindgen_export_4: WebAssembly.Table;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_export_6: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const closure128_externref_shim: (a: number, b: number, c: any) => void;
export const closure158_externref_shim: (a: number, b: number, c: any, d: any) => void;
export const __wbindgen_start: () => void;
