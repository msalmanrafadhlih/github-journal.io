use super::data::Chronicle;
use super::util::format_short_date;

const WIDTH: f64 = 1200.0;
const HEIGHT: f64 = 400.0;
const TOOLTIP_W: f64 = 180.0;
const TOOLTIP_H: f64 = 90.0;

/// Builds the full `<svg>...</svg>` markup for the contribution timeline,
/// including the gradient area, stroke path, and collision-avoided tooltips
/// for each entry in `top_activities`. Rendered via `dangerous_inner_html`
/// so we don't need typed RSX bindings for `<foreignObject>`/`<linearGradient>`.
pub fn build_chart_svg(chronicle: &Chronicle) -> String {
    let timeline = &chronicle.stats.timeline;

    let mut svg = format!(
        r#"<svg class="w-full h-full overflow-visible" preserveAspectRatio="none" viewBox="0 0 {WIDTH} {HEIGHT}" xmlns="http://www.w3.org/2000/svg">"#
    );
    svg.push_str(
        r##"<defs><linearGradient id="gradient-fill" x1="0" x2="0" y1="0" y2="1"><stop offset="0%" stop-color="#C94B28" stop-opacity="0.1"></stop><stop offset="100%" stop-color="#C94B28" stop-opacity="0"></stop></linearGradient></defs>"##,
    );

    if timeline.is_empty() {
        svg.push_str("</svg>");
        return svg;
    }

    let max_count = timeline.iter().map(|d| d.count).max().unwrap_or(1).max(1) as f64;
    let step_x = WIDTH / (timeline.len() as f64 - 1.0).max(1.0);

    let points: Vec<(f64, f64)> = timeline
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let x = i as f64 * step_x;
            let y = HEIGHT - (d.count as f64 / max_count) * (HEIGHT - 50.0);
            (x, y)
        })
        .collect();

    let mut d_path = format!("M {},{}", points[0].0, points[0].1);
    for pair in points.windows(2) {
        let (p0x, p0y) = pair[0];
        let (p1x, p1y) = pair[1];
        let cp1x = p0x + (p1x - p0x) / 2.0;
        let cp2x = cp1x;
        d_path.push_str(&format!(" C {cp1x},{p0y} {cp2x},{p1y} {p1x},{p1y}"));
    }

    let area_path = format!("{d_path} L {WIDTH},{HEIGHT} L 0,{HEIGHT} Z");
    svg.push_str(&format!(
        r#"<path d="{area_path}" fill="url(#gradient-fill)"></path>"#
    ));
    svg.push_str(&format!(
        r##"<path d="{d_path}" fill="none" stroke="#1A1A1A" stroke-width="2"></path>"##
    ));

    if let Some(activities) = &chronicle.top_activities {
        let mut placed: Vec<(f64, f64, f64, f64)> = Vec::new(); // (x, y, w, h)

        for activity in activities {
            let Some(idx) = timeline
                .iter()
                .position(|t| format_short_date(&t.date) == activity.date)
            else {
                continue;
            };
            let (cx, cy) = points[idx];

            svg.push_str(r#"<g class="cursor-pointer group/point">"#);
            svg.push_str(&format!(
                r##"<circle cx="{cx}" cy="{cy}" r="4" fill="#1A1A1A"></circle>"##
            ));
            svg.push_str(&format!(
                r##"<line x1="{cx}" x2="{cx}" y1="{cy}" y2="{HEIGHT}" stroke="#C94B28" stroke-dasharray="4 4" stroke-width="1"></line>"##
            ));

            let mut fx = cx - TOOLTIP_W / 2.0;
            let mut fy = cy - TOOLTIP_H - 15.0;
            if fx < 0.0 {
                fx = 0.0;
            }
            if fx + TOOLTIP_W > WIDTH {
                fx = WIDTH - TOOLTIP_W;
            }

            let mut collision = true;
            let mut attempts = 0;
            while collision && attempts < 10 {
                collision = false;
                for &(rx, ry, rw, rh) in &placed {
                    if fx < rx + rw && fx + TOOLTIP_W > rx && fy < ry + rh && fy + TOOLTIP_H > ry {
                        collision = true;
                        break;
                    }
                }
                if collision {
                    if attempts == 0 {
                        fy = cy + 15.0;
                    } else if attempts == 1 {
                        fy = cy - TOOLTIP_H - 80.0;
                    } else if attempts < 5 {
                        fx -= TOOLTIP_W + 10.0;
                    } else {
                        fy -= TOOLTIP_H + 10.0;
                    }
                    if fx < 0.0 {
                        fx = 0.0;
                    }
                    attempts += 1;
                }
            }
            placed.push((fx, fy, TOOLTIP_W, TOOLTIP_H));

            svg.push_str(&format!(
                r#"<foreignObject x="{fx}" y="{fy}" width="{TOOLTIP_W}" height="{TOOLTIP_H}"><div xmlns="http://www.w3.org/1999/xhtml" class="bg-primary text-paper p-4 shadow-2xl border border-white/10"><div class="font-serif italic text-lg opacity-80 mb-1">{}</div><div class="font-mono text-xl font-bold tracking-tight">{} Commits</div></div></foreignObject>"#,
                activity.date, activity.count
            ));
            svg.push_str("</g>");
        }
    }

    svg.push_str("</svg>");
    svg
}
