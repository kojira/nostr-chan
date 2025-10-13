import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { readFileSync } from 'fs'
import yaml from 'js-yaml'

// config.ymlを読み込む
let dashboardPort = 3000; // デフォルト値
try {
  const config = yaml.load(readFileSync('../config.yml', 'utf8'));
  dashboardPort = config.dashboard?.port || 3000;
} catch (e) {
  console.warn('config.ymlの読み込みに失敗。デフォルトポート3000を使用します。');
}

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3001,
    proxy: {
      '/api': {
        target: `http://localhost:${dashboardPort}`,
        changeOrigin: true,
      }
    }
  },
  build: {
    outDir: '../dashboard',
    emptyOutDir: true,
  }
})
