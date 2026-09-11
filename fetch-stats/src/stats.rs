//! Fetches GitHub contribution/repo data via the GraphQL API and reduces it
//! into the exact JSON shape `main/src/journal/data.rs::AppData` expects.
//!
//! This is a direct port of the query-building and date-math logic from the
//! sibling `github-profile.svg` generator's `src/stats.rs` (same account,
//! same GraphQL query) — only the output shape differs, since that project
//! renders `window.profileData` into an HTML template for a screenshot,
//! while this one writes plain `data.json` for the live Dioxus site to
//! `fetch()` at runtime.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::config::CONFIG;

#[derive(Serialize)]
pub struct Hero {
    pub total_repos: i64,
    pub total_stars: i64,
    pub total_followers: i64,
}

#[derive(Serialize)]
pub struct Repo {
    pub title: String,
    pub description: String,
    pub stars: i64,
    pub topic: String,
    pub language: String,
}

#[derive(Serialize, Clone)]
pub struct LanguageStat {
    pub name: String,
    pub percent: f64,
    pub color: String,
}

#[derive(Serialize, Clone)]
pub struct DayPoint {
    pub date: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct PeakDay {
    pub date: String,
}

#[derive(Serialize)]
pub struct Stats {
    pub timeline: Vec<DayPoint>,
}

#[derive(Serialize)]
pub struct Chronicle {
    pub total_contribution_volume: i64,
    pub growth_percentage: String,
    pub current_streak: i64,
    pub peak_activity_day: PeakDay,
    pub monthly_focus: String,
    pub monthly_focus_html: String,
    pub most_productive_day: String,
    pub languages: Vec<LanguageStat>,
    pub stack: Vec<String>,
    pub stats: Stats,
    pub top_activities: Vec<DayPoint>,
}

#[derive(Serialize)]
pub struct AppData {
    pub hero: Hero,
    pub pinned: Vec<Repo>,
    pub chronicle: Chronicle,
}

fn get_activity_type(language: Option<&str>) -> &'static str {
    match language {
        None => "general development",
        Some(lang) => match lang {
            "Rust" => "systems programming",
            "Go" => "backend services",
            "TypeScript" => "application development",
            "JavaScript" => "interactive interfaces",
            "Python" => "data processing",
            "Lua" => "configuration & scripting",
            "Nix" => "reproducible infrastructure",
            "HTML" => "structure & layout",
            "CSS" => "visual styling",
            "C++" => "performance engineering",
            "C" => "low-level system logic",
            "Shell" => "automation scripts",
            _ => "coding activity",
        },
    }
}

/// JS `new Date(year, monthIndexAbs, 1)`, normalized the same way the JS
/// `Date` constructor rolls over months >= 12 (or negative) into later/
/// earlier years. `month_index_abs` is 0-indexed, like `Date.getMonth()`,
/// but may be outside 0..=11.
fn month_start(year: i32, month_index_abs: i32) -> NaiveDate {
    let real_year = year + month_index_abs.div_euclid(12);
    let real_month0 = month_index_abs.rem_euclid(12);
    NaiveDate::from_ymd_opt(real_year, (real_month0 + 1) as u32, 1)
        .expect("normalized year/month should always be valid")
}

/// JS `new Date(year, monthIndexAbs, 0)` — day 0 means "the day before day
/// 1", i.e. the last day of the previous month.
fn day_zero(year: i32, month_index_abs: i32) -> NaiveDate {
    month_start(year, month_index_abs) - chrono::Duration::days(1)
}

/// Formats like JS `Date.prototype.toISOString()`: `YYYY-MM-DDTHH:mm:ss.sssZ`.
fn to_iso_string(date: NaiveDate, hms: (u32, u32, u32)) -> String {
    let dt = Utc.from_utc_datetime(&date.and_hms_opt(hms.0, hms.1, hms.2).unwrap());
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn build_query() -> String {
    let pinned_repos_query: String = CONFIG
        .pinned
        .iter()
        .enumerate()
        .map(|(index, repo)| {
            format!(
                r#"
  repo{index}: repository(owner: $username, name: "{name}") {{
    name
    description
    stargazerCount
    primaryLanguage {{
      name
      color
    }}
  }}
"#,
                index = index,
                name = repo.name
            )
        })
        .collect();

    format!(
        r#"
  query($username: String!, $currentYearStart: DateTime!, $currentYearEnd: DateTime!, $lastYearStart: DateTime!, $lastYearEnd: DateTime!) {{
    user(login: $username) {{
      name
      bio
      followers {{
        totalCount
      }}
      repositories(first: 100, ownerAffiliations: OWNER, isFork: false, orderBy: {{field: STARGAZERS, direction: DESC}}) {{
        totalCount
        nodes {{
          name
          stargazerCount
          primaryLanguage {{
            name
            color
          }}
          languages(first: 10, orderBy: {{field: SIZE, direction: DESC}}) {{
            edges {{
              size
              node {{
                name
                color
              }}
            }}
          }}
        }}
      }}
      currentYear: contributionsCollection(from: $currentYearStart, to: $currentYearEnd) {{
        contributionCalendar {{
          totalContributions
          weeks {{
            contributionDays {{
              contributionCount
              date
              weekday
            }}
          }}
        }}
        commitContributionsByRepository(maxRepositories: 100) {{
          repository {{
            name
            primaryLanguage {{
              name
            }}
          }}
          contributions(first: 100) {{
            nodes {{
              occurredAt
              commitCount
            }}
          }}
        }}
      }}
      lastYear: contributionsCollection(from: $lastYearStart, to: $lastYearEnd) {{
        contributionCalendar {{
          totalContributions
        }}
      }}
    }}
    {pinned_repos_query}
  }}
"#
    )
}

/// Blocking on purpose: this is a linear, single-shot CI script (fetch once,
/// write the file, exit), so there's no benefit to an async HTTP client here
/// — `ureq` keeps the dependency tree (and MSRV) far lighter than `reqwest`.
fn fetch_data(token: &str, username: &str) -> Result<Value> {
    let now = Utc::now();
    let (y, m0) = (now.year(), now.month0() as i32); // month0: 0-indexed like JS getMonth()

    let current_year_start = to_iso_string(month_start(y - 1, m0 + 1), (0, 0, 0));
    let current_year_end = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let last_year_start = to_iso_string(month_start(y - 2, m0 + 1), (0, 0, 0));
    let last_year_end = to_iso_string(day_zero(y - 1, m0 + 1), (0, 0, 0));

    let body = serde_json::json!({
        "query": build_query(),
        "variables": {
            "username": username,
            "currentYearStart": current_year_start,
            "currentYearEnd": current_year_end,
            "lastYearStart": last_year_start,
            "lastYearEnd": last_year_end,
        }
    });

    let data: Value = ureq::post("https://api.github.com/graphql")
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", "fetch-stats")
        .send_json(body)
        .context("request to GitHub GraphQL API failed")?
        .into_json()
        .context("failed to parse GraphQL response as JSON")?;

    if let Some(errors) = data.get("errors") {
        bail!("GraphQL error: {}", serde_json::to_string_pretty(errors)?);
    }

    data.get("data")
        .cloned()
        .context("GraphQL response missing `data` field")
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn i(v: &Value) -> i64 {
    v.as_i64().unwrap_or(0)
}

/// Reads `GITHUB_TOKEN` (or `TOKEN`) from the environment, fetches live
/// stats, and reduces them into `AppData`.
pub fn generate_stats() -> Result<AppData> {
    let token = std::env::var("TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .context("GITHUB_TOKEN (or TOKEN) env var is missing")?;

    let data = fetch_data(&token, CONFIG.username)?;
    let user = data.get("user").context("missing `user` in response")?;

    let repo_nodes = user["repositories"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let total_stars: i64 = repo_nodes.iter().map(|r| i(&r["stargazerCount"])).sum();

    let hero = Hero {
        total_repos: i(&user["repositories"]["totalCount"]),
        total_stars,
        total_followers: i(&user["followers"]["totalCount"]),
    };

    let pinned: Vec<Repo> = CONFIG
        .pinned
        .iter()
        .enumerate()
        .filter_map(|(index, cfg_repo)| {
            let repo = data.get(format!("repo{index}"))?;
            if repo.is_null() {
                return None;
            }
            Some(Repo {
                title: s(&repo["name"]),
                description: repo["description"]
                    .as_str()
                    .filter(|d| !d.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| "No description yet.".to_string()),
                stars: i(&repo["stargazerCount"]),
                topic: cfg_repo.topic.to_string(),
                language: repo["primaryLanguage"]["name"]
                    .as_str()
                    .unwrap_or("N/A")
                    .to_string(),
            })
        })
        .collect();

    // Aggregate language byte-size across all owned repos.
    let mut language_stats: BTreeMap<String, (i64, String)> = BTreeMap::new();
    let mut total_size: i64 = 0;
    for repo in &repo_nodes {
        if let Some(edges) = repo["languages"]["edges"].as_array() {
            for edge in edges {
                let size = i(&edge["size"]);
                let name = s(&edge["node"]["name"]);
                let color = s(&edge["node"]["color"]);
                let entry = language_stats.entry(name).or_insert((0, color));
                entry.0 += size;
                total_size += size;
            }
        }
    }
    let mut languages: Vec<LanguageStat> = language_stats
        .into_iter()
        .map(|(name, (size, color))| LanguageStat {
            name,
            percent: if total_size > 0 {
                (size as f64 / total_size as f64 * 1000.0).round() / 10.0
            } else {
                0.0
            },
            color,
        })
        .collect();
    languages.sort_by(|a, b| b.percent.partial_cmp(&a.percent).unwrap());
    languages.truncate(5);

    let current_total = i(&user["currentYear"]["contributionCalendar"]["totalContributions"]);
    let last_year_total = i(&user["lastYear"]["contributionCalendar"]["totalContributions"]);
    let growth_raw = if last_year_total > 0 {
        (current_total - last_year_total) as f64 / last_year_total as f64 * 100.0
    } else {
        0.0
    };
    let growth_percentage = if growth_raw >= 0.0 {
        format!("+{growth_raw:.1}%")
    } else {
        format!("{growth_raw:.1}%")
    };

    #[derive(Clone)]
    struct Day {
        date: String,
        count: i64,
        weekday: i64,
    }
    let mut days: Vec<Day> = Vec::new();
    if let Some(weeks) = user["currentYear"]["contributionCalendar"]["weeks"].as_array() {
        for week in weeks {
            if let Some(cdays) = week["contributionDays"].as_array() {
                for d in cdays {
                    days.push(Day {
                        date: s(&d["date"]),
                        count: i(&d["contributionCount"]),
                        weekday: i(&d["weekday"]),
                    });
                }
            }
        }
    }

    let today_str = Utc::now().format("%Y-%m-%d").to_string();
    let mut today_index = days.iter().position(|d| d.date == today_str);
    if today_index.is_none() {
        today_index = if days.is_empty() {
            None
        } else {
            Some(days.len() - 1)
        };
    }

    let mut current_streak: i64 = 0;
    if let Some(start_idx) = today_index {
        let mut i_idx = start_idx as i64;
        while i_idx >= 0 {
            let idx = i_idx as usize;
            if days[idx].count > 0 {
                current_streak += 1;
            } else if idx == start_idx && days[idx].count == 0 {
                // today with zero contributions yet — doesn't break the streak
            } else {
                break;
            }
            i_idx -= 1;
        }
    }

    let mut sorted_days = days.clone();
    sorted_days.sort_by(|a, b| b.count.cmp(&a.count)); // stable sort, matches JS engines

    let peak_day = sorted_days.first().cloned().unwrap_or(Day {
        date: today_str.clone(),
        count: 0,
        weekday: 0,
    });

    let format_short_date = |date_str: &str| -> String {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map(|d| d.format("%b %-d").to_string())
            .unwrap_or_else(|_| date_str.to_string())
    };

    let peak_activity_day = PeakDay {
        date: format_short_date(&peak_day.date),
    };

    let top_activities: Vec<DayPoint> = sorted_days
        .iter()
        .take(3)
        .map(|d| DayPoint {
            date: format_short_date(&d.date),
            count: d.count,
        })
        .collect();

    let current_month0 = Utc::now().month0() as i64; // 0-indexed, matches JS getMonth()
    let current_days: Vec<&Day> = days
        .iter()
        .filter(|d| {
            NaiveDate::parse_from_str(&d.date, "%Y-%m-%d")
                .map(|nd| (nd.month0() as i64) == current_month0)
                .unwrap_or(false)
        })
        .collect();

    let mut day_counts = [0i64; 7];
    for d in &current_days {
        let idx = d.weekday.clamp(0, 6) as usize;
        day_counts[idx] += d.count;
    }
    const DAYS_OF_WEEK: [&str; 7] = [
        "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
    ];
    let mut best_day = DAYS_OF_WEEK[Utc::now().weekday().num_days_from_sunday() as usize];
    let mut max_count = -1i64;
    for (idx, &count) in day_counts.iter().enumerate() {
        if count > max_count {
            max_count = count;
            best_day = DAYS_OF_WEEK[idx];
        }
    }

    // Ordered like JS object insertion order: first-seen repo keeps its position.
    let mut repo_counts: Vec<(String, i64)> = Vec::new();
    let mut repo_languages: BTreeMap<String, String> = BTreeMap::new();
    if let Some(contribs) = user["currentYear"]["commitContributionsByRepository"].as_array() {
        for rc in contribs {
            let repo_name = s(&rc["repository"]["name"]);
            if let Some(lang) = rc["repository"]["primaryLanguage"]["name"].as_str() {
                repo_languages.insert(repo_name.clone(), lang.to_string());
            }
            if let Some(nodes) = rc["contributions"]["nodes"].as_array() {
                for node in nodes {
                    let occurred = s(&node["occurredAt"]);
                    let month0 = occurred
                        .get(0..7)
                        .and_then(|prefix| {
                            NaiveDate::parse_from_str(&format!("{prefix}-01"), "%Y-%m-%d").ok()
                        })
                        .map(|d| d.month0() as i64);
                    if month0 == Some(current_month0) {
                        let commit_count = i(&node["commitCount"]);
                        match repo_counts.iter_mut().find(|(n, _)| n == &repo_name) {
                            Some(entry) => entry.1 += commit_count,
                            None => repo_counts.push((repo_name.clone(), commit_count)),
                        }
                    }
                }
            }
        }
    }
    let mut focus_sorted = repo_counts.clone();
    focus_sorted.sort_by(|a, b| b.1.cmp(&a.1)); // stable
    let monthly_focus = focus_sorted
        .first()
        .map(|(n, _)| n.clone())
        .unwrap_or_else(|| "Research".to_string());

    let monthly_focus_html = if let Some((top_repo, _)) = focus_sorted.first() {
        let top_lang = repo_languages.get(top_repo).cloned();
        let activity_type = get_activity_type(top_lang.as_deref());
        let month_name = Utc::now().format("%B").to_string();
        format!(
            r#"{month_name} saw a significant shift towards {activity_type}, with heavy activity in <span class="text-accent font-medium">{lang}</span> configurations for the <span class="text-accent font-medium">{repo}</span> setup."#,
            lang = top_lang.unwrap_or_else(|| "Code".to_string()),
            repo = top_repo,
        )
    } else {
        "Structuring <b>ideas</b> into reality.".to_string()
    };

    let chronicle = Chronicle {
        total_contribution_volume: current_total,
        growth_percentage,
        current_streak,
        peak_activity_day,
        monthly_focus,
        monthly_focus_html,
        most_productive_day: best_day.to_string(),
        languages,
        stack: CONFIG.stack.iter().map(|s| s.to_string()).collect(),
        stats: Stats {
            timeline: days
                .into_iter()
                .map(|d| DayPoint {
                    date: d.date,
                    count: d.count,
                })
                .collect(),
        },
        top_activities,
    };

    Ok(AppData {
        hero,
        pinned,
        chronicle,
    })
}
