# Github Journal

A Dioxus (Rust/WASM) rebuild of the `github-profile.svg` README card — same
layout, but a real interactive website with **live data pulled from the
GitHub API** instead of a static screenshot, plus scroll-triggered reveal
animations.

## How it fits together

```
fetch-stats/   Rust CLI. Queries the GitHub GraphQL API for your profile,
               pinned repos, and contribution calendar, then writes
               data.json shaped exactly like main/src/journal/data.rs
               expects (ported from the github-profile.svg generator's
               src/stats.rs — same query, same account).

main/          The Dioxus site. On load it fetches data.json; if that
               isn't reachable yet (e.g. very first local run before any
               CI run has produced one) it falls back to the bundled
               main/data/journal.sample.json so the page never breaks.

.github/workflows/deploy.yml
               Runs fetch-stats -> builds main/ with `dx bundle` -> ships
               the result to GitHub Pages. Triggers on every push to
               main, every 6 hours on a schedule (so stats stay fresh
               even with no new commits), and manually via
               "Run workflow".
```

## One-time repo setup

1. Push this to a GitHub repo.
2. **Settings → Pages → Build and deployment → Source: "GitHub Actions"**.
   (Without this the workflow's `actions/deploy-pages` step has nothing to
   deploy to.)
3. That's it — no secrets to add. The workflow uses the default
   `GITHUB_TOKEN` that Actions already provides, which is enough to read
   public profile/contribution data over GraphQL.
   - If you'd rather also count **private** contributions in the total,
     create a classic PAT with the `read:user` scope, add it as a repo
     secret named `STATS_TOKEN`, and the workflow will use that instead
     (see the `env:` line in `deploy.yml`).

The account/pinned-repos/stack shown on the card are configured in
`fetch-stats/src/config.rs` — edit that file (not the Dioxus source) if
your pinned repos or stack change.

## Local development

```bash
# Site with the bundled sample data (no token needed):
cd main
dx serve

# Site with real data:
cd fetch-stats && GITHUB_TOKEN=<a classic PAT> cargo run -- ../main/data.json
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
