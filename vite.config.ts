import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'

export default defineConfig({
  plugins: [solid()],
  server: {
    port: 5173,
    strictPort: true,
    host: process.env.TAURI_DEV_HOST || false,
  },
  build: {
    chunkSizeWarningLimit: 5000,
  },
})
