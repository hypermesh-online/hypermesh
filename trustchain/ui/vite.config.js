import routify from '@roxi/routify/vite-plugin'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'
import { resolve } from 'path'

const production = process.env.NODE_ENV === 'production'

export default defineConfig({
    clearScreen: false,
    resolve: { 
        alias: { 
            '@': resolve('src'),
            '$lib': resolve('src/lib')
        } 
    },
    plugins: [
        routify({
            render: {
                ssr: { enable: false },
            },
        }),
        svelte({
            compilerOptions: {
                dev: !production,
                hydratable: !!process.env.ROUTIFY_SSR_ENABLE,
                runes: true, // Enable Svelte 5 runes
            },
            extensions: ['.svelte'],
        }),
    ],
    server: { 
        port: 1337,
        host: '::',  // IPv6 support for Web3 ecosystem
    },
    define: {
        'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV)
    }
})
