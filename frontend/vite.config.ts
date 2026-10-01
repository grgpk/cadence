import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { nitro } from "nitro/vite";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("./", import.meta.url));

export default defineConfig({
  server: { port: 3000, strictPort: true },
  resolve: { alias: { "@": `${root}src/`, "~": `${root}src/` } },
  plugins: [tailwindcss(), tanstackStart({ srcDirectory: "src" }), react(), nitro()],
});
