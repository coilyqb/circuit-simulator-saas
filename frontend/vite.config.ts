import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    lib: { entry: 'src/core/index.ts', formats: ['es'], fileName: 'core' },
  },
})
