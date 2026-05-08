// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { defineConfig, loadEnv } from 'vite'
import path from 'path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'

export default defineConfig(({ mode }) => {
  // Load env file based on `mode` in the current working directory.
  const env = loadEnv(mode, process.cwd(), '')

  return {
    resolve: {
      alias: {
        '@': path.resolve(__dirname),
        '@core': path.resolve(__dirname, '../..'),
      },
    },
    plugins: [
      tailwindcss(),
      react(),
    ],
    build: {
      outDir: 'dist',
      assetsDir: 'assets',
      sourcemap: mode === 'development',
      minify: mode === 'production' ? 'esbuild' : false,
      rollupOptions: {
        // Phase C.3 — `@tauri-apps/api/*` is loaded only when running
        // inside the Tauri desktop bundle (where Tauri injects the
        // module at runtime). For the standalone Gateway-served build
        // we mark it external so Vite/Rollup doesn't try to resolve
        // it. The dynamic `import()` in components/wizard/tauriBridge.ts
        // is wrapped in a try/catch and gated by an `isTauri()` check,
        // so missing modules at runtime are a graceful no-op.
        external: [/^@tauri-apps\/api(\/.*)?$/],
        output: {
          // Ensure consistent chunk naming
          chunkFileNames: 'assets/[name]-[hash].js',
          entryFileNames: 'assets/[name]-[hash].js',
          assetFileNames: 'assets/[name]-[hash].[ext]',
        },
      },
    },
    server: {
      port: 5173,
      host: true,
      cors: true,
    },
    preview: {
      port: 4173,
      host: true,
    },
  }
})
