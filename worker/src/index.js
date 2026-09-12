/**
 * Cloudflare Worker replacing the old `fetch-stats` CI job.
 *
 * Instead of a scheduled GitHub Action writing a static `data.json` every
 * 6 hours, this Worker runs the *same* GraphQL query on-demand, at the
 * edge, and hands back JSON in the exact shape `main/src/data.rs::AppData`
 * expects. The Dioxus site fetches this endpoint directly at runtime.
 *
 * This is a line-for-line port of `fetch-stats/src/{config,stats}.rs` —
 * see that file's history in git if you need to cross-check behavior.
 * Two simplifications were possible during the port:
 *   - The Rust code manually re-implemented JS's `Date` month-rollover
 *     math (see its `month_start`/`day_zero` comments) specifically so it
 *     would match this exact behavior — so here we just use `Date` directly.
 *   - Rust's `serde_json::Value` needed explicit `.as_str()`/`.as_i64()`
 *     helpers; plain JS objects don't.
 *
 * Caching: responses are cached at Cloudflare's edge (Cache API) for
 * `CACHE_TTL_SECONDS` (default 300s / 5 min). Within that window, repeat
 * visits are served from cache and never touch the GitHub API at all —
 * this is what keeps usage well inside both GitHub's GraphQL rate limit
 * and the Workers free tier.
 */

// ---------------------------------------------------------------------
// Config — same account/pinned-repos/stack that used to live in
// fetch-stats/src/config.rs. Edit here if they change.
// ---------------------------------------------------------------------
const CONFIG = {
  username: "msalmanrafadhlih",
  pinned: [
    { name: "flexinix", topic: "Nixos Configuration Flakes" },
    { name: "racooonfig", topic: "Linux Dotfiles" },
    { name: "template.nix", topic: "Nix Development Templates" },
    { name: "fastfetch.md", topic: "termux config" },
    { name: "tquilla.is-a.bot", topic: "discord bot" },
    { name: "pocket-ai.gemini", topic: "project exam" },
  ],
  stack: ["NixOS", "Rust", "Helix", "Figma", "Canva"],
};

const ACTIVITY_TYPES = {
  Rust: "systems programming",
  Go: "backend services",
  TypeScript: "application development",
  JavaScript: "interactive interfaces",
  Python: "data processing",
  Lua: "configuration & scripting",
  Nix: "reproducible infrastructure",
  HTML: "structure & layout",
  CSS: "visual styling",
  "C++": "performance engineering",
  C: "low-level system logic",
  Shell: "automation scripts",
};

function getActivityType(language) {
  if (!language) return "general development";
  return ACTIVITY_TYPES[language] || "coding activity";
}

// ---------------------------------------------------------------------
// GraphQL query (identical shape to the old Rust `build_query`)
// ---------------------------------------------------------------------
function buildQuery(pinned) {
  const pinnedReposQuery = pinned
    .map(
      (repo, index) => `
  repo${index}: repository(owner: $username, name: "${repo.name}") {
    name
    description
    stargazerCount
    primaryLanguage {
      name
      color
    }
  }
`
    )
    .join("");

  return `
  query($username: String!, $currentYearStart: DateTime!, $currentYearEnd: DateTime!, $lastYearStart: DateTime!, $lastYearEnd: DateTime!) {
    user(login: $username) {
      name
      bio
      followers {
        totalCount
      }
      repositories(first: 100, ownerAffiliations: OWNER, isFork: false, orderBy: {field: STARGAZERS, direction: DESC}) {
        totalCount
        nodes {
          name
          stargazerCount
          primaryLanguage {
            name
            color
          }
          languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
            edges {
              size
              node {
                name
                color
              }
            }
          }
        }
      }
      currentYear: contributionsCollection(from: $currentYearStart, to: $currentYearEnd) {
        contributionCalendar {
          totalContributions
          weeks {
            contributionDays {
              contributionCount
              date
              weekday
            }
          }
        }
        commitContributionsByRepository(maxRepositories: 100) {
          repository {
            name
            primaryLanguage {
              name
            }
          }
          contributions(first: 100) {
            nodes {
              occurredAt
              commitCount
            }
          }
        }
      }
      lastYear: contributionsCollection(from: $lastYearStart, to: $lastYearEnd) {
        contributionCalendar {
          totalContributions
        }
      }
    }
    ${pinnedReposQuery}
  }
`;
}

function monthStartUTC(year, monthIndexAbs) {
  return new Date(Date.UTC(year, monthIndexAbs, 1));
}

function dayZeroUTC(year, monthIndexAbs) {
  return new Date(Date.UTC(year, monthIndexAbs, 0));
}

async function fetchGraphQL(token, config) {
  const now = new Date();
  const y = now.getUTCFullYear();
  const m0 = now.getUTCMonth(); // 0-indexed, like Rust's month0()

  const variables = {
    username: config.username,
    currentYearStart: monthStartUTC(y - 1, m0 + 1).toISOString(),
    currentYearEnd: now.toISOString(),
    lastYearStart: monthStartUTC(y - 2, m0 + 1).toISOString(),
    lastYearEnd: dayZeroUTC(y - 1, m0 + 1).toISOString(),
  };

  const res = await fetch("https://api.github.com/graphql", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "User-Agent": "github-journal-worker",
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query: buildQuery(config.pinned), variables }),
  });

  if (!res.ok) {
    throw new Error(`GitHub API responded ${res.status} ${res.statusText}`);
  }

  const json = await res.json();
  if (json.errors) {
    throw new Error(`GraphQL error: ${JSON.stringify(json.errors)}`);
  }
  if (!json.data) {
    throw new Error("GraphQL response missing `data` field");
  }
  return json.data;
}

// ---------------------------------------------------------------------
// Reduce the raw GraphQL response into main/src/data.rs::AppData's shape
// (identical field names/types to the old Rust `generate_stats`).
// ---------------------------------------------------------------------
function formatShortDate(dateStr) {
  const d = new Date(`${dateStr}T00:00:00Z`);
  if (Number.isNaN(d.getTime())) return dateStr;
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}

function transformStats(data, config) {
  const user = data.user;
  if (!user) throw new Error("missing `user` in GraphQL response");

  const repoNodes = user.repositories?.nodes ?? [];
  const totalStars = repoNodes.reduce((sum, r) => sum + (r.stargazerCount ?? 0), 0);

  const hero = {
    total_repos: user.repositories?.totalCount ?? 0,
    total_stars: totalStars,
    total_followers: user.followers?.totalCount ?? 0,
  };

  // Pinned repos — a name that doesn't exist under the account (private,
  // renamed, deleted) comes back `null` and is skipped silently, same as
  // the old Rust `filter_map`.
  const pinned = config.pinned
    .map((cfgRepo, index) => {
      const repo = data[`repo${index}`];
      if (!repo) return null;
      const description =
        typeof repo.description === "string" && repo.description.length > 0
          ? repo.description
          : "No description yet.";
      return {
        title: repo.name ?? "",
        description,
        stars: repo.stargazerCount ?? 0,
        topic: cfgRepo.topic,
        language: repo.primaryLanguage?.name ?? "N/A",
      };
    })
    .filter(Boolean);

  // Aggregate language byte-size across all owned repos.
  const languageStats = new Map(); // name -> { size, color }
  let totalSize = 0;
  for (const repo of repoNodes) {
    for (const edge of repo.languages?.edges ?? []) {
      const size = edge.size ?? 0;
      const name = edge.node?.name ?? "";
      const color = edge.node?.color ?? "";
      const entry = languageStats.get(name) ?? { size: 0, color };
      entry.size += size;
      languageStats.set(name, entry);
      totalSize += size;
    }
  }
  let languages = Array.from(languageStats.entries()).map(([name, { size, color }]) => ({
    name,
    percent: totalSize > 0 ? Math.round((size / totalSize) * 1000) / 10 : 0,
    color,
  }));
  languages.sort((a, b) => b.percent - a.percent);
  languages = languages.slice(0, 5);

  const currentTotal = user.currentYear?.contributionCalendar?.totalContributions ?? 0;
  const lastYearTotal = user.lastYear?.contributionCalendar?.totalContributions ?? 0;
  const growthRaw = lastYearTotal > 0 ? ((currentTotal - lastYearTotal) / lastYearTotal) * 100 : 0;
  const growthPercentage = `${growthRaw >= 0 ? "+" : ""}${growthRaw.toFixed(1)}%`;

  const days = [];
  for (const week of user.currentYear?.contributionCalendar?.weeks ?? []) {
    for (const d of week.contributionDays ?? []) {
      days.push({
        date: d.date,
        count: d.contributionCount ?? 0,
        weekday: d.weekday ?? 0,
      });
    }
  }

  const todayStr = new Date().toISOString().slice(0, 10);
  let todayIndex = days.findIndex((d) => d.date === todayStr);
  if (todayIndex === -1) {
    todayIndex = days.length > 0 ? days.length - 1 : -1;
  }

  let currentStreak = 0;
  if (todayIndex !== -1) {
    for (let idx = todayIndex; idx >= 0; idx--) {
      if (days[idx].count > 0) {
        currentStreak += 1;
      } else if (idx === todayIndex) {
        // Today with zero contributions yet doesn't break the streak.
      } else {
        break;
      }
    }
  }

  // Array#sort is a stable sort in every modern engine (incl. workerd/V8),
  // matching the "stable sort" comment on the original Rust code.
  const sortedDays = [...days].sort((a, b) => b.count - a.count);
  const peakDay = sortedDays[0] ?? { date: todayStr, count: 0, weekday: 0 };
  const peakActivityDay = { date: formatShortDate(peakDay.date) };
  const topActivities = sortedDays
    .slice(0, 3)
    .map((d) => ({ date: formatShortDate(d.date), count: d.count }));

  const currentMonth0 = new Date().getUTCMonth();
  const dayCounts = new Array(7).fill(0);
  for (const d of days) {
    const nd = new Date(`${d.date}T00:00:00Z`);
    if (Number.isNaN(nd.getTime()) || nd.getUTCMonth() !== currentMonth0) continue;
    const idx = Math.min(Math.max(d.weekday, 0), 6);
    dayCounts[idx] += d.count;
  }
  const DAYS_OF_WEEK = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
  let bestDay = DAYS_OF_WEEK[new Date().getUTCDay()];
  let maxCount = -1;
  dayCounts.forEach((count, idx) => {
    if (count > maxCount) {
      maxCount = count;
      bestDay = DAYS_OF_WEEK[idx];
    }
  });

  // Commit counts by repo, this month — insertion-ordered, like the old
  // Vec<(String, i64)> the Rust side used instead of a plain HashMap.
  const repoCounts = [];
  const repoCountIndex = new Map();
  const repoLanguages = new Map();
  for (const rc of user.currentYear?.commitContributionsByRepository ?? []) {
    const repoName = rc.repository?.name ?? "";
    const lang = rc.repository?.primaryLanguage?.name;
    if (lang) repoLanguages.set(repoName, lang);

    for (const node of rc.contributions?.nodes ?? []) {
      const occurred = node.occurredAt ?? "";
      const monthDate = new Date(`${occurred.slice(0, 7)}-01T00:00:00Z`);
      if (Number.isNaN(monthDate.getTime()) || monthDate.getUTCMonth() !== currentMonth0) continue;

      const commitCount = node.commitCount ?? 0;
      if (repoCountIndex.has(repoName)) {
        repoCounts[repoCountIndex.get(repoName)].count += commitCount;
      } else {
        repoCountIndex.set(repoName, repoCounts.length);
        repoCounts.push({ name: repoName, count: commitCount });
      }
    }
  }

  const focusSorted = [...repoCounts].sort((a, b) => b.count - a.count);
  const monthlyFocus = focusSorted[0]?.name ?? "Research";

  let monthlyFocusHtml;
  if (focusSorted.length > 0) {
    const topRepo = focusSorted[0].name;
    const topLang = repoLanguages.get(topRepo);
    const activityType = getActivityType(topLang);
    const monthName = new Date().toLocaleDateString("en-US", { month: "long", timeZone: "UTC" });
    monthlyFocusHtml =
      `${monthName} saw a significant shift towards ${activityType}, with heavy activity in ` +
      `<span class="text-accent font-medium">${topLang ?? "Code"}</span> configurations for the ` +
      `<span class="text-accent font-medium">${topRepo}</span> setup.`;
  } else {
    monthlyFocusHtml = "Structuring <b>ideas</b> into reality.";
  }

  const chronicle = {
    total_contribution_volume: currentTotal,
    growth_percentage: growthPercentage,
    current_streak: currentStreak,
    peak_activity_day: peakActivityDay,
    monthly_focus: monthlyFocus,
    monthly_focus_html: monthlyFocusHtml,
    most_productive_day: bestDay,
    languages,
    stack: config.stack,
    stats: {
      timeline: days.map((d) => ({ date: d.date, count: d.count })),
    },
    top_activities: topActivities,
  };

  return { hero, pinned, chronicle };
}

// ---------------------------------------------------------------------
// HTTP handler: cache lookup -> (on miss) GraphQL fetch + transform ->
// cache write -> CORS-wrapped JSON response.
// ---------------------------------------------------------------------
function corsHeaders(env) {
  return {
    "Access-Control-Allow-Origin": env.ALLOWED_ORIGIN || "*",
    "Access-Control-Allow-Methods": "GET, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
  };
}

export default {
  async fetch(request, env, ctx) {
    if (request.method === "OPTIONS") {
      return new Response(null, { headers: corsHeaders(env) });
    }

    const url = new URL(request.url);
    if (url.pathname !== "/" && url.pathname !== "/api/stats") {
      return new Response("Not found", { status: 404, headers: corsHeaders(env) });
    }

    // Cloudflare's edge Cache API. Keyed on a fixed URL (not the real
    // request URL) so every visitor hits the same cache entry regardless
    // of query params or which path they used.
    const cache = caches.default;
    const cacheKey = new Request("https://github-journal-stats.internal/api/stats", { method: "GET" });

    const cached = await cache.match(cacheKey);
    if (cached) {
      const res = new Response(cached.body, cached);
      for (const [k, v] of Object.entries(corsHeaders(env))) res.headers.set(k, v);
      res.headers.set("X-Cache", "HIT");
      return res;
    }

    if (!env.GITHUB_TOKEN) {
      return new Response(JSON.stringify({ error: "GITHUB_TOKEN secret is not configured" }), {
        status: 500,
        headers: { "content-type": "application/json", ...corsHeaders(env) },
      });
    }

    try {
      const raw = await fetchGraphQL(env.GITHUB_TOKEN, CONFIG);
      const appData = transformStats(raw, CONFIG);
      const ttl = parseInt(env.CACHE_TTL_SECONDS || "300", 10);

      const response = new Response(JSON.stringify(appData), {
        status: 200,
        headers: {
          "content-type": "application/json; charset=utf-8",
          "cache-control": `public, max-age=${ttl}`,
          "X-Cache": "MISS",
          ...corsHeaders(env),
        },
      });

      // Store the cacheable version (without CORS headers baked in isn't
      // necessary here — they're static per env — but we re-set them on
      // every read anyway in case ALLOWED_ORIGIN changes after a deploy).
      ctx.waitUntil(cache.put(cacheKey, response.clone()));
      return response;
    } catch (err) {
      return new Response(JSON.stringify({ error: String(err?.message ?? err) }), {
        status: 502,
        headers: { "content-type": "application/json", ...corsHeaders(env) },
      });
    }
  },
};
