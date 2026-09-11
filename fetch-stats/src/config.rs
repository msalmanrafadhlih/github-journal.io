//! Static configuration for which GitHub account/repos feed the site.
//!
//! Ported from the `github-profile.svg` generator's `src/config.rs` so both
//! projects stay in sync. Edit the values below if the account or the
//! pinned repos change.

pub struct PinnedRepo {
    pub name: &'static str,
    pub topic: &'static str,
}

pub struct Config {
    pub username: &'static str,
    pub pinned: &'static [PinnedRepo],
    pub stack: &'static [&'static str],
}

// A name that doesn't exist under the account (private, renamed, deleted)
// is skipped silently by the GraphQL query below (not an error) — the
// PinnedSection component already renders fine with fewer than 6 cards.
pub const CONFIG: Config = Config {
    username: "msalmanrafadhlih",

    pinned: &[
        PinnedRepo { name: "flexinix", topic: "Nixos Configuration Flakes" },
        PinnedRepo { name: "racooonfig", topic: "Linux Dotfiles" },
        PinnedRepo { name: "template.nix", topic: "Nix Development Templates" },
        PinnedRepo { name: "fastfetch.md", topic: "termux config" },
        PinnedRepo { name: "tquilla.is-a.bot", topic: "discord bot" },
        PinnedRepo { name: "pocket-ai.gemini", topic: "project exam" },
    ],

    // Shown in the "Core Stack (Detected)" chip row on the card.
    stack: &["NixOS", "Rust", "Helix", "Figma", "Canva"],
};
