import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Configurazione consigliata da Tauri: porta fissa, nessun watch sulla parte Rust.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: { target: "es2022", sourcemap: false },
});
