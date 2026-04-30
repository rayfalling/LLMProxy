# LLMProxy Dashboard UI

React + TypeScript frontend for the LLMProxy admin dashboard.

## Development

```bash
npm install
npm run dev
```

Server runs on http://localhost:5173 (dev mode proxies `/api` to http://localhost:8081)

## Build

```bash
npm run build
```

Outputs to `dist/` directory for embedding in Rust dashboard binary.

## Structure

- `src/pages/` - Page components (Login, Setup, Dashboard, CRUD pages)
- `src/components/` - Reusable components (Header, ProtectedRoute, etc)
- `src/api/` - API client and TypeScript types
- `vite.config.ts` - Vite build configuration
- `tailwind.config.js` - Tailwind CSS theme
