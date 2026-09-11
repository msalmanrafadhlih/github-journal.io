mod chart;
mod components;
mod data;
mod scroll;
mod util;

use dioxus::prelude::*;

use components::{ChronicleSection, EndOfStream, Footer, Hero, PinnedSection};
use data::AppData;

/// Bundled at compile time so the page always has something to render, even
/// before `data.json` is fetched — or if that fetch fails for any reason.
/// `data.json` itself is written next to `index.html` by the `fetch-stats`
/// crate, running on a schedule in `.github/workflows/deploy.yml` (the same
/// pattern the `github-profile.svg` generator uses for its own stats).
const SAMPLE_DATA: &str = include_str!("../../data/journal.sample.json");
const FAVICON: &str = "https://avatars.githubusercontent.com/u/141149698";

async fn load_data() -> Result<AppData, String> {
    // Relative (no leading `/`) so this resolves correctly whether the site
    // is served from the domain root or from a GitHub Pages project
    // subpath (e.g. `username.github.io/repo-name/data.json`).
    if let Ok(resp) = gloo_net::http::Request::get("data.json").send().await {
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
pub fn JournalPage() -> Element {
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
