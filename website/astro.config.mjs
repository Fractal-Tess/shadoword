import { defineConfig } from 'astro/config'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  output: 'static',
  /**
   * The deployed host, and it is load-bearing rather than cosmetic: `canonical`,
   * `og:url` and `og:image` are all absolute against it, so a wrong value does
   * not degrade gracefully — every share card resolves its image against a
   * domain that does not exist and renders as a bare link. This was
   * `https://shadoword.dev`, which was an assumption nobody had registered.
   */
  site: 'https://shadoword.fractal-tess.xyz',
  trailingSlash: 'never',
  vite: {
    plugins: [tailwindcss()],
  },
})
