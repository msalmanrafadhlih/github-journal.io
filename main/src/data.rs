use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AppData {
    pub hero: Hero,
    pub pinned: Vec<Repo>,
    pub chronicle: Chronicle,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Hero {
    pub total_repos: u32,
    pub total_stars: u32,
    pub total_followers: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Repo {
    pub title: String,
    pub description: String,
    pub stars: u32,
    #[serde(default)]
    pub topic: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Chronicle {
    pub total_contribution_volume: u32,
    pub growth_percentage: String,
    pub current_streak: u32,
    pub peak_activity_day: PeakDay,
    pub monthly_focus: String,
    #[serde(default)]
    pub monthly_focus_html: Option<String>,
    pub most_productive_day: String,
    pub languages: Vec<LanguageStat>,
    #[serde(default)]
    pub stack: Option<Vec<String>>,
    pub stats: Stats,
    #[serde(default)]
    pub top_activities: Option<Vec<Activity>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PeakDay {
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LanguageStat {
    pub name: String,
    pub percent: f64,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Stats {
    pub timeline: Vec<TimelinePoint>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TimelinePoint {
    pub date: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Activity {
    pub date: String,
    pub count: u32,
}
