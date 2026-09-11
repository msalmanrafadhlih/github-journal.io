mod config;
mod stats;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let out_path = std::env::args().nth(1).unwrap_or_else(|| "data.json".to_string());

    eprintln!("[1/2] fetching live stats from the GitHub GraphQL API...");
    let data = stats::generate_stats().context("failed to generate stats")?;

    eprintln!("[2/2] writing {out_path}...");
    let json = serde_json::to_string_pretty(&data).context("failed to serialize stats to JSON")?;
    std::fs::write(&out_path, json).with_context(|| format!("failed to write {out_path}"))?;

    eprintln!("done — wrote {out_path}");
    Ok(())
}
