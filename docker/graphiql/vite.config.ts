import react from "@vitejs/plugin-react";
import {defineConfig} from "vite";
import sri from '@vividlemon/vite-plugin-sri'
export default defineConfig({
    plugins: [
        react(),
        sri()
    ],
    build: {
        sourcemap: true,
        minify: true,
        cssMinify: true,
        chunkSizeWarningLimit: 1000,
    },
    server: {
        warmup: {
            clientFiles: [
                "src/main.tsx",
                "src/App.tsx",
                "src/GraphiQL.tsx",
            ],
        },
    },
});
