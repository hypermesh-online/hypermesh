// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/// <reference types="vite/client" />

// Node.js globals for config files (vite, vitest, playwright)
declare var process: {
  env: Record<string, string | undefined>;
  cwd(): string;
};
declare var __dirname: string;
declare var __filename: string;
declare var global: typeof globalThis;

declare module 'path' {
  export function resolve(...paths: string[]): string;
  export function join(...paths: string[]): string;
  export function dirname(path: string): string;
  export function basename(path: string, ext?: string): string;
  export function extname(path: string): string;
  export function relative(from: string, to: string): string;
  export function isAbsolute(path: string): boolean;
  export function normalize(path: string): string;
  const _default: {
    resolve: typeof resolve;
    join: typeof join;
    dirname: typeof dirname;
    basename: typeof basename;
    extname: typeof extname;
    relative: typeof relative;
    isAbsolute: typeof isAbsolute;
    normalize: typeof normalize;
  };
  export default _default;
}

// NodeJS namespace for timer types
declare namespace NodeJS {
  interface Timeout {}
}
