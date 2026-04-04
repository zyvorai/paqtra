import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/__tests__/setup.ts',
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:9191',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:9191',
        ws: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          'react-vendor': ['react', 'react-dom', 'react-router-dom'],
          'query-vendor': ['@tanstack/react-query', '@tanstack/react-table'],
          'chart-vendor': ['recharts'],
          'icon-vendor': ['lucide-react'],
        },
      },
    },
  },
});
