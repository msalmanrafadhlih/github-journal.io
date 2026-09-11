use super::data::TimelinePoint;

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const MONTHS_UPPER: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// Mirrors the JS `formatStars` helper: 1234 -> "1.2K", 42 -> "42".
pub fn format_stars(count: u32) -> String {
    if count < 1000 {
        count.to_string()
    } else {
        format!("{:.1}K", count as f64 / 1000.0)
    }
}

/// Mirrors `c.total_contribution_volume.toLocaleString()` (en-US thousands grouping).
pub fn format_volume(n: u32) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i != 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    grouped.chars().rev().collect()
}

/// Formats an ISO `YYYY-MM-DD` date the way `toLocaleDateString("en-US", { month: "short", day: "numeric" })` does.
pub fn format_short_date(iso_date: &str) -> String {
    let parts: Vec<&str> = iso_date.split('-').collect();
    if parts.len() < 3 {
        return iso_date.to_string();
    }
    let month_idx = parts[1]
        .parse::<usize>()
        .unwrap_or(1)
        .saturating_sub(1)
        .min(11);
    let day: u32 = parts[2].parse().unwrap_or(1);
    format!("{} {}", MONTHS_SHORT[month_idx], day)
}

/// Rolling 12-month label row starting from the first timeline entry's month,
/// same as the JS `months` generation.
pub fn build_month_labels(timeline: &[TimelinePoint]) -> Vec<String> {
    let Some(first) = timeline.first() else {
        return vec![];
    };
    let parts: Vec<&str> = first.date.split('-').collect();
    let start_month = parts
        .get(1)
        .and_then(|m| m.parse::<usize>().ok())
        .unwrap_or(1)
        .saturating_sub(1)
        % 12;
    (0..12)
        .map(|i| MONTHS_SHORT[(start_month + i) % 12].to_string())
        .collect()
}

/// Today's date, computed in-browser via `js_sys::Date` (equivalent to `new Date()`).
pub struct Today {
    pub month_upper: &'static str,
    pub day: u32,
    pub year: i32,
    pub day_of_year: u32,
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn today() -> Today {
    let now = js_sys::Date::new_0();
    let month0 = now.get_month() as usize; // 0-indexed, like JS
    let day = now.get_date() as u32;
    let year = now.get_full_year() as i32;

    const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut day_of_year = day;
    for m in 0..month0 {
        day_of_year += DAYS_IN_MONTH[m];
        if m == 1 && is_leap_year(year) {
            day_of_year += 1;
        }
    }

    Today {
        month_upper: MONTHS_UPPER[month0],
        day,
        year,
        day_of_year,
    }
}
