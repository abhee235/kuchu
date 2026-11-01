import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// The pages Tauri shows: served on a fixed port while developing, built to dist/ for the app.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: '127.0.0.1' },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: { target: 'es2022', outDir: 'dist' },
})
