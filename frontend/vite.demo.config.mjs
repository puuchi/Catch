import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

const relative = (path) => fileURLToPath(new URL(path, import.meta.url));

export default defineConfig({
  root: relative('../site/demo/'),
  publicDir: relative('./public/'),
  base: './',
  plugins: [
    {
      name: 'catch-browser-host',
      enforce: 'pre',
      resolveId(source, importer) {
        if (importer && /^\.\.?\/bridge$/.test(source)) return relative('../site/demo/browser.js');
        if (source === '@tauri-apps/api/webview') return '\0catch-browser-webview';
      },
      load(id) {
        if (id === '\0catch-browser-webview') {
          return 'export const getCurrentWebview = () => ({ onDragDropEvent: async () => () => {} });';
        }
      },
    },
    vue(),
  ],
  build: {
    target: 'es2022',
    outDir: relative('../site/desktop-demo/'),
    emptyOutDir: true,
    copyPublicDir: false,
  },
});
