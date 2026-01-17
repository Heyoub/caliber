# CALIBER Landing Page

Landing page and dashboard for CALIBER - Memory for AI Agents.

Built with Astro 5 + Svelte 5 + Tailwind CSS 4.

## Getting Started

```bash
# Install dependencies
bun install

# Start dev server
bun run dev

# Type-check
bun run typecheck

# Build for production
bun run build

# Preview production build
bun run preview
```

## Project Structure

```text
/
├── public/
│   ├── favicon.svg
│   └── ...
├── src/
│   ├── components/
│   │   ├── svelte/          # Svelte 5 components
│   │   └── *.astro          # Astro components
│   ├── layouts/
│   │   ├── Layout.astro     # Marketing layout
│   │   └── DashboardLayout.astro  # Dashboard layout
│   ├── lib/
│   │   └── api.ts           # API client
│   ├── pages/
│   │   ├── index.astro      # Landing page
│   │   ├── login.astro      # Login page
│   │   ├── auth/callback.astro  # OAuth callback
│   │   └── dashboard/       # Dashboard pages
│   ├── stores/
│   │   └── auth.ts          # Auth state management
│   └── styles/
│       └── global.css       # Global styles
└── package.json
```

## Commands

| Command           | Action                                      |
|:------------------|:--------------------------------------------|
| `bun install`     | Install dependencies                        |
| `bun run dev`     | Start dev server at `localhost:4321`        |
| `bun run build`   | Build production site to `./dist/`          |
| `bun run preview` | Preview build locally                       |
| `bun run test`    | Run Playwright tests                        |
| `bun run typecheck` | Run TypeScript type checking              |

## npm Compatibility

For npm users, replace `bun` with `npm`:

```bash
npm install
npm run dev
npm run build
```

## Environment Variables

Create a `.env` file:

```env
PUBLIC_API_URL=https://api.caliber.run
```

## License

AGPL-3.0-or-later
