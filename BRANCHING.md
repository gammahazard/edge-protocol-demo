# Git Flow Branching Strategy

This project follows Git Flow for structured development and deployment.

## Branches

| Branch | Purpose | Deploys To |
|:-------|:--------|:-----------|
| `main` | Production-ready code | `edge-protocol-demo.workers.dev` |
| `develop` | Integration branch | `develop.edge-protocol-demo.workers.dev` |
| `feature/*` | New features | Local only (PR to develop) |
| `hotfix/*` | Urgent production fixes | Merge to both main and develop |

## Workflow

```
main ──────────────────────────────────────► Production
  │                                              ▲
  ▼                                              │
develop ───────────────────────────────────► Preview
  │         ▲         ▲         ▲
  ▼         │         │         │
feature/*   feature/* feature/* hotfix/*
```

### Starting a Feature

```bash
git checkout develop
git pull origin develop
git checkout -b feature/add-modbus-parser
# ... work ...
git push origin feature/add-modbus-parser
# Create PR to develop
```

### Releasing to Production

```bash
git checkout main
git merge develop
git push origin main
# Cloudflare auto-deploys
git checkout develop
```

### Hotfix

```bash
git checkout main
git checkout -b hotfix/fix-parsing-bug
# ... fix ...
git checkout main && git merge hotfix/fix-parsing-bug
git checkout develop && git merge hotfix/fix-parsing-bug
git branch -d hotfix/fix-parsing-bug
```
