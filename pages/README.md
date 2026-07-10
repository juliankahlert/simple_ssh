# Vue.js Documentation App

Interactive documentation viewer for the simple_ssh Rust library.

## Prerequisites

- Node.js 18+

## Installation

```bash
npm install
```

## Development

Start the development server:

```bash
npm run dev
```

This command also copies Rust examples from the parent `examples/` directory to `public/examples/` and regenerates `src/composables/examplesData.ts` before starting the dev server.

## Build

Create a production build:

```bash
npm run build
```

Preview the production build locally:

```bash
npm run preview
```

## Project Structure

```text
pages/
├── src/
│   ├── components/    # Vue components
│   ├── composables/   # Vue composables
│   ├── styles/        # Style files
│   └── assets/        # Static assets
├── scripts/           # Build and utility scripts
├── public/            # Public static assets
│   └── examples/      # Copied Rust examples (auto-generated)
└── tests/             # Vitest tests
```

## Adding/Modifying Examples

1. Add or modify Rust example files in the parent `examples/` directory
2. Run `npm run copy-examples` to copy them to `public/examples/`
3. The examples will be available in the documentation app

## Deployment

The project is automatically deployed via GitHub Actions on push to the main branch. The workflow builds the Vue.js app and deploys it to GitHub Pages.

