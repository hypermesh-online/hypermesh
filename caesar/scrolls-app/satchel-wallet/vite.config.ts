// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { visualizer } from 'rollup-plugin-visualizer'

export default defineConfig({
  plugins: [
    react(),
    // Bundle analysis plugin - only run in analysis mode
    process.env.ANALYZE === 'true' && visualizer({
      filename: 'bundle-analysis/bundle-report.html',
      open: false,
      gzipSize: true,
      brotliSize: true,
      template: 'treemap', // 'treemap' | 'sunburst' | 'network'
    }),
  ].filter(Boolean),
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3002,
    host: true
  },
  define: {
    global: 'globalThis',
  },
  build: {
    // Enable source maps for analysis
    sourcemap: process.env.ANALYZE === 'true',
    // Report compressed size
    reportCompressedSize: true,
    // Chunk size warning limit (default 500)
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        // Manual chunks for better analysis
        manualChunks: {
          // Vendor chunks
          react: ['react', 'react-dom'],
          web3: ['ethers', 'viem', 'wagmi', '@web3modal/wagmi'],
          ui: ['@headlessui/react', '@heroicons/react', 'lucide-react'],
          utils: ['clsx', 'tailwind-merge', 'date-fns'],
          query: ['@tanstack/react-query'],
          qr: ['qrcode', 'react-qr-code'],
          plaid: ['react-plaid-link'],
        },
      },
    },
  },
})