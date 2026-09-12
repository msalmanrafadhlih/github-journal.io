# Github Journal

A Dioxus (Rust/WASM) rebuild of the `github-profile.svg` README card — same
layout, but a real interactive website with **live data pulled from the
GitHub API at runtime** instead of a static screenshot, plus scroll-triggered
reveal animations.

## How it fits together

```
worker/        Cloudflare Worker. Runs the GitHub GraphQL query (profile,
                pinned repos, contribution calendar) on-demand and returns
                JSON shaped exactly like main/src/data.rs expects. Caches
                each response at the edge for a few minutes (Cache API) so
                repeat visits never re-hit GitHub's API. Replaces what used
                to be a `fetch-stats` Rust CLI + a 6-hourly CI cron.

main/           The Dioxus site. On load it fetches STATS_API_URL (the
                deployed Worker) directly from the browser; if that fails
                for any reason, it falls back to the bundled
                main/data/journal.sample.json so the page never breaks.

.github/workflows/deploy.yml
                Builds main/ with `dx bundle` -> ships the result to
                GitHub Pages. Triggers on push to main (ignoring worker/
                changes) and manually via "Run workflow". No cron anymore
                — there's nothing left to periodically refresh, since data
                is fetched live on every page load.

.github/workflows/deploy-worker.yml
                Deploys worker/ to Cloudflare Workers whenever that folder
                changes, via `wrangler deploy`.
```

### Why a Worker instead of a build-time `data.json`

The card needs the contribution-calendar heatmap and pinned repos, both of
which only exist in GitHub's **GraphQL** API — and GraphQL requires an auth
token, which can't be embedded in client-side WASM/JS without leaking it to
anyone who opens dev tools. The Worker holds that token server-side and
exposes a plain JSON endpoint the browser can call anonymously; a short edge
cache keeps it fast and keeps usage well within GitHub's and Cloudflare's
free-tier limits. See `worker/README.md` for the full rationale.

## One-time setup

**1. Deploy the Worker first** — full instructions in `worker/README.md`,
short version:
```bash
cd worker
npm install
npx wrangler login
npx wrangler secret put GITHUB_TOKEN   # a classic PAT with read:user scope
npm run deploy
```
Wrangler prints a URL like `https://github-journal-stats.<subdomain>.workers.dev`.

**2. Point the site at it** — paste that URL into `STATS_API_URL` near the
top of `main/src/mod.rs`, replacing the placeholder.

**3. Push this repo to GitHub, then Settings → Pages → Build and
deployment → Source: "GitHub Actions".**

**4. (Optional, for auto-deploying the Worker from CI)** add two repo
secrets under Settings → Secrets and variables → Actions:
- `CLOUDFLARE_API_TOKEN` — needs "Edit Cloudflare Workers" permission
- `CLOUDFLARE_ACCOUNT_ID`

Without these two secrets, `deploy-worker.yml` simply won't run
successfully — you can always fall back to `npm run deploy` from your own
machine instead.

The account/pinned-repos/stack shown on the card are configured in the
`CONFIG` object at the top of `worker/src/index.js` — edit that (not the
Dioxus source) if your pinned repos or stack change, then redeploy the
Worker.

## Local development

```bash
# Site with the bundled sample data (no Worker needed):
cd main
dx serve

# Site with real data — run the Worker locally in one terminal
# (needs worker/.dev.vars with a GITHUB_TOKEN — see worker/README.md):
cd worker && npm run dev
# ...and point STATS_API_URL at http://localhost:8787/api/stats
# temporarily while developing, then:
cd ../main && dx serve
```

`main/Dioxus.toml` deliberately leaves `base_path` unset for this reason —
setting it breaks `dx serve` locally. The deploy workflow injects the
correct value (your repo name, or `/` if the repo is a `*.github.io` user
page) right before the production build, so you never need to hand-edit it.

## Scroll-triggered animations

Implemented with a native `IntersectionObserver`
(`main/src/journal/scroll.rs`) — no extra crates. Elements marked
`data-reveal` in `components.rs` start faded/offset (CSS in `index.html`);
the first time one scrolls into view, `is-visible` is added to its class
list, which runs the transition, and that element is then unobserved (a
one-shot reveal, not a repeat-on-every-scroll effect). Respects
`prefers-reduced-motion`.

To animate more sections, add `"data-reveal": "true"` to any element in
`components.rs` — nothing else needs to change.
