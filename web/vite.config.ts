import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../static',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/api': {
        target: 'https://zcgzoso3znn9kl-3000.proxy.runpod.net',
        changeOrigin: true,
      },
    },
  },
})
