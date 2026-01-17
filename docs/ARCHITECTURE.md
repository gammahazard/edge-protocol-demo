# Architecture Overview

## System Design

This project demonstrates production-style Cloudflare Workers patterns with a Rust WASM dashboard.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CLOUDFLARE PAGES                                     │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    Leptos WASM Dashboard                             │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐                     │   │
│  │  │URL Shortener│  │Rate Limiter│  │ Capability │                     │   │
│  │  │    Tab     │  │    Tab     │  │  Explorer  │                     │   │
│  │  └────────────┘  └────────────┘  └────────────┘                     │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                │ fetch()
                                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CLOUDFLARE WORKERS (300+ locations)                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │  url-shortener  │  │  rate-limiter   │  │ capability-demo │             │
│  │  POST /shorten  │  │ GET /api/protect│  │ GET /api/capab. │             │
│  │  GET /:code     │  │ GET /api/status │  │                 │             │
│  │                 │  │                 │  │ Tests: fetch,   │             │
│  │ Creates short   │  │ Sliding window  │  │ kv, filesystem, │             │
│  │ URLs via KV     │  │ rate limiting   │  │ sockets, exec   │             │
│  └────────┬────────┘  └────────┬────────┘  └─────────────────┘             │
│           │                    │                                            │
│           ▼                    ▼                                            │
│  ┌─────────────────────────────────────────┐                               │
│  │            Workers KV Storage            │                               │
│  │   URLS namespace   RATES namespace       │                               │
│  └─────────────────────────────────────────┘                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Workers

| Worker | Purpose | Storage |
|:-------|:--------|:--------|
| **url-shortener** | Create/redirect short URLs with click tracking | KV (`URLS`) |
| **rate-limiter** | Sliding window rate limiting with standard headers | KV (`RATES`) |
| **capability-demo** | Demonstrate Workers security sandbox | None |

## Capability Model

Workers use the same **capability-based security** as WASI:

| Capability | Status | Reason |
|:-----------|:-------|:-------|
| `fetch()` | ✅ Allowed | HTTP requests via Fetch API |
| `KV Storage` | ✅ Allowed | When bound in wrangler.toml |
| `Filesystem` | ❌ Blocked | No fs module, no File API |
| `Raw Sockets` | ❌ Blocked | Only fetch(), no TCP/UDP |
| `Subprocess` | ❌ Blocked | No exec, no shell access |

## Project Structure

```
edge-protocol-demo/
├── dashboard/               # Leptos WASM web UI
│   ├── src/
│   │   ├── lib.rs          # App + routing
│   │   ├── api.rs          # Worker API client
│   │   └── components/     # Tab components
│   └── dist/               # Built WASM output (Pages)
│
├── workers/
│   ├── url-shortener/      # KV-backed URL shortening
│   ├── rate-limiter/       # Edge rate limiting
│   └── capability-demo/    # Security sandbox demo
│
├── shared/                 # Common Rust types
├── .github/workflows/      # CI/CD (deploys workers)
└── docs/                   # This documentation
```

## Request Flow

```
1. User visits https://edge-protocol-demo.pages.dev
2. Leptos WASM app loads in browser
3. User interacts (e.g., shortens a URL)
4. Dashboard calls Worker API via fetch()
5. Worker executes Rust WASM on nearest edge node
6. Response returns with data + CORS headers
7. Dashboard updates UI + persists to localStorage
```

## Deployment

| Branch | Pages URL | Workers |
|:-------|:----------|:--------|
| `develop` | develop.edge-protocol-demo.pages.dev | *-preview.workers.dev |
| `main` | edge-protocol-demo.pages.dev | *.workers.dev |

> **Dashboard Build Note:** The dashboard is built locally with `trunk build --release` and the `dashboard/dist/` folder is committed to Git. Cloudflare Pages serves these pre-built files directly (no build step configured). After making changes to `dashboard/src/**`, you must rebuild and commit the dist folder for changes to deploy.

## Tech Stack

| Layer | Technology |
|:------|:-----------|
| **Dashboard** | Rust + Leptos 0.7 (CSR) |
| **Workers** | Rust + workers-rs 0.7 |
| **Build** | Trunk (dashboard), worker-build (workers) |
| **Storage** | Workers KV |
| **CI/CD** | GitHub Actions |
