mod chart;
mod components;
mod data;
mod scroll;
mod util;

use dioxus::prelude::*;

use components::{ChronicleSection, EndOfStream, Footer, Hero, PinnedSection};
use data::AppData;

/// Bundled at compile time so the page always has something to render
/// immediately, and as a fallback if the live fetch below fails for any
/// reason (Worker cold-started wrong, network hiccup, rate-limited, etc.)
/// — the page should never show a broken/empty state.
const SAMPLE_DATA: &str = include_str!("../../data/journal.sample.json");
const FAVICON: &str = "https://avatars.githubusercontent.com/u/141149698";

/// The Cloudflare Worker that now does what the old `fetch-stats` CI job +
/// `data.json` used to: run the GitHub GraphQL query and hand back JSON in
/// this exact shape, cached at the edge for a few minutes. See
/// `worker/README.md` for setup/deploy instructions.
///
/// This has to be a full absolute URL (unlike the old relative
/// `data.json` path) since it points at a different origin than the
/// GitHub Pages site itself. Update this after your first
/// `wrangler deploy` in `worker/`.
const STATS_API_URL: &str = "https://github-journal-stats.msalmanrafadhlih.workers.dev/api/stats";

fn main() {
    dioxus::launch(App);
}

async fn load_data() -> Result<AppData, String> {
    if let Ok(resp) = gloo_net::http::Request::get(STATS_API_URL).send().await {
        if resp.ok() {
            if let Ok(text) = resp.text().await {
                if let Ok(parsed) = serde_json::from_str::<AppData>(&text) {
                    return Ok(parsed);
                }
            }
        }
    }
    serde_json::from_str::<AppData>(SAMPLE_DATA).map_err(|e| e.to_string())
}

#[component]
pub fn App() -> Element {
    let mut data: Signal<Option<AppData>> = use_signal(|| None);
    let mut load_error: Signal<Option<String>> = use_signal(|| None);

    use_effect(move || {
        spawn(async move {
            match load_data().await {
                Ok(parsed) => data.set(Some(parsed)),
                Err(e) => load_error.set(Some(e)),
            }
        });
    });

    rsx! {
        if let Some(d) = data() {
            Page { data: d }
        } else if let Some(err) = load_error() {
            div { class: "min-h-screen flex items-center justify-center bg-paper text-primary font-mono text-sm",
                "Failed to load data: {err}"
            }
        } else {
            div { class: "min-h-screen flex items-center justify-center bg-paper text-primary font-mono text-sm",
                "Loading…"
            }
        }
    }
}

#[component]
fn Page(data: AppData) -> Element {
    let today = util::today();
    let date_label = format!("{} {}", today.month_upper, today.day);
    let build_number = format!("NO. {}", today.day_of_year);

    // Runs once, right after this page's real DOM (all `[data-reveal]`
    // sections) is mounted — see `scroll::init_scroll_reveal` for how the
    // actual observing/animating works.
    use_effect(move || {
        scroll::init_scroll_reveal();
    });

    rsx! {
        document::Title { "Github Journal" }
        document::Link { rel: "icon", href: FAVICON }
        div { class: "bg-paper text-primary font-sans antialiased flex flex-col selection:bg-accent selection:text-paper overflow-x-hidden",
            Hero { hero: data.hero.clone(), date_label, build_number }
            PinnedSection { pinned: data.pinned.clone() }
            ChronicleSection { chronicle: data.chronicle.clone() }
            EndOfStream {}
            Footer { year: today.year }
        }
    }
}
