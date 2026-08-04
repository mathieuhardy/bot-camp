import path from 'node:path'

import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  // Served under /dashboard by the Rust backend (src/routes/dashboard.rs),
  // not from the site root.
  base: '/dashboard/',
  plugins: [tailwindcss(), svelte()],
  resolve: {
    alias: {
      $lib: path.resolve(import.meta.dirname, './src/lib'),
    },
  },
  server: {
    // Forward API/WebSocket calls to a bot-camp instance running
    // locally, so `npm run dev` gets live data instead of just serving
    // the static shell.
    proxy: {
      '/dashboard/snapshot': 'http://127.0.0.1:3000',
      '/dashboard/ws': { target: 'http://127.0.0.1:3000', ws: true },
    },
  },
})
