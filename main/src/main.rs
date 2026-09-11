use dioxus::prelude::*;

mod journal;
use journal::JournalPage;

const FAVICON: &str = "https://avatars.githubusercontent.com/u/141149698";

fn main() {
    dioxus::launch(App);
}

#[component]
pub fn App() -> Element {
    rsx! {
        JournalPage {}
    }
}

