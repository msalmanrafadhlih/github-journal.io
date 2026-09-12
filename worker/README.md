# github-journal-stats (Cloudflare Worker)

Replaces the old `fetch-stats` Rust CLI + 6-hourly cron. Runs the same
GitHub GraphQL query on-demand, at the edge, with a short edge cache —
so the site gets live data on every visit without any scheduled CI job.

## One-time setup

1. **Install deps** (from this `worker/` folder):
   ```bash
   npm install
   ```

2. **Log in to Cloudflare** (opens a browser once):
   ```bash
   npx wrangler login
   ```

3. **Create a GitHub token** the Worker will use server-side (never
   exposed to the browser):
   - Classic PAT with `read:user` scope is enough for public data.
   - Add `repo` scope too if you want private contributions counted.

4. **Store the token as a Worker secret** (encrypted, not in git):
   ```bash
   npx wrangler secret put GITHUB_TOKEN
   # paste the token when prompted
   ```

5. **Deploy**:
   ```bash
   npm run deploy
   ```
   Wrangler prints your Worker's URL, e.g.
   `https://github-journal-stats.<your-subdomain>.workers.dev`.

6. **Point the site at it** — put that URL into `STATS_API_URL` in
   `main/src/mod.rs` (see the comment right above it), then redeploy the
   Dioxus site as usual.

7. *(Optional, tighter CORS)* once you know your GitHub Pages URL, set
   `ALLOWED_ORIGIN` in `wrangler.toml` to that exact origin instead of
   `"*"` and redeploy the Worker.

## Local development

`wrangler secret put` only applies to the **deployed** Worker — local dev
doesn't see it. For local testing, create a `worker/.dev.vars` file
(dotenv-style, already gitignored — never commit this):

```bash
echo 'GITHUB_TOKEN=<a classic PAT with read:user scope>' > .dev.vars
npm run dev
```

Wrangler loads `.dev.vars` automatically and serves the Worker at
`http://localhost:8787` — hit `http://localhost:8787/api/stats` to see the
JSON it returns.

## Why the Cache API, and why this stays inside the free tier

- GitHub's GraphQL API allows **5,000 points/hour** with a token — a
  personal-profile query like this one costs only a handful of points,
  so even without caching it wouldn't get close to that limit for normal
  traffic.
- The Cache API entry is what keeps it cheap regardless: within the
  `CACHE_TTL_SECONDS` window (default 5 minutes), *every* visitor is
  served the same cached response straight from Cloudflare's edge — zero
  extra calls to GitHub's API, and zero extra Worker CPU time beyond
  reading the cache.
- Cloudflare Workers' free plan includes 100,000 requests/day, which is
  far more than a personal profile page needs.

## Editing pinned repos / stack / username

Edit the `CONFIG` object at the top of `src/index.js` (this replaces what
used to live in `fetch-stats/src/config.rs`), then `npm run deploy` again.
