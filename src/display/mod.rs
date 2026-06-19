use colored::{ColoredString, Colorize};

use crate::{
    backends::{PackageInfo, PackageSource},
    trust::TrustReport,
};

const MAX_SEARCH_RESULTS: usize = 40;

pub fn print_search_results(results: &[PackageInfo]) {
    if results.is_empty() {
        println!("No packages found.");
        return;
    }

    println!(
        "{:<32} {:<14} {:<16} {}",
        "Package".cyan().bold(),
        "Version".cyan().bold(),
        "Source".cyan().bold(),
        "Description".cyan().bold()
    );

    for pkg in results.iter().take(MAX_SEARCH_RESULTS) {
        println!(
            "{:<32} {:<14} {:<16} {}",
            pkg.name.as_str().bold(),
            truncate(&pkg.version, 14),
            source_badge(&pkg.source),
            truncate(&pkg.description, 80)
        );
    }

    if results.len() > MAX_SEARCH_RESULTS {
        println!(
            "{}",
            format!(
                "... {} more result(s). Refine the query to narrow the list.",
                results.len() - MAX_SEARCH_RESULTS
            )
            .dimmed()
        );
    }
}

pub fn print_trust_report(pkg: &PackageInfo, report: &TrustReport) {
    println!(
        "{} {}",
        "Package:".cyan().bold(),
        report.package_name.as_str().bold()
    );
    println!("{} {}", "Version:".cyan().bold(), pkg.version);
    println!("{} {}", "Repository:".cyan().bold(), pkg.source);
    println!("{} {}", "Trust Tier:".cyan().bold(), report.tier);

    if let Some(maintainer) = &pkg.maintainer {
        println!("{} {}", "Maintainer:".cyan().bold(), maintainer);
    }
    if let Some(url) = &pkg.url {
        println!("{} {}", "URL:".cyan().bold(), url);
    }
    if let Some(popularity) = pkg.popularity {
        println!("{} {:.2}", "Popularity:".cyan().bold(), popularity);
    }
    if let Some(install_size) = &pkg.install_size {
        println!("{} {}", "Installed Size:".cyan().bold(), install_size);
    }
    if !pkg.licenses.is_empty() {
        println!("{} {}", "Licenses:".cyan().bold(), pkg.licenses.join(", "));
    }
    if !pkg.depends.is_empty() {
        println!(
            "{} {}",
            "Dependencies:".cyan().bold(),
            summarize_list(&pkg.depends, 8)
        );
    }
    if !pkg.description.is_empty() {
        println!("{} {}", "Description:".cyan().bold(), pkg.description);
    }

    println!();
    println!(
        "{} {}",
        "Trust Score:".cyan().bold(),
        colored_score(report.score).bold()
    );
    println!(
        "{} {}",
        "Recommendation:".cyan().bold(),
        colored_recommendation(&report.recommendation)
    );

    println!();
    println!("{}", "Signals".cyan().bold());
    for signal in &report.signals {
        let points = if signal.points >= 0 {
            format!("+{}/{}", signal.points, signal.max_points)
        } else {
            signal.points.to_string()
        };

        println!(
            "  {:<20} {:>7}  {}",
            signal.name.as_str().bold(),
            color_points(signal.points, &points),
            signal.detail
        );
    }
}

fn source_badge(source: &PackageSource) -> ColoredString {
    match source {
        PackageSource::Official { repo } => format!("official/{}", repo).green(),
        PackageSource::Aur => "aur".yellow(),
        PackageSource::ThirdParty => "third-party".magenta(),
        PackageSource::Unknown => "unknown".normal(),
    }
}

fn colored_score(score: u32) -> ColoredString {
    let text = format!("{}/100", score);
    match score {
        70..=100 => text.green(),
        40..=69 => text.yellow(),
        _ => text.red(),
    }
}

fn colored_recommendation(recommendation: &str) -> ColoredString {
    match recommendation {
        "Safe" | "Moderate" => recommendation.green(),
        "Caution" => recommendation.yellow(),
        _ => recommendation.red(),
    }
}

fn color_points(points: i32, text: &str) -> ColoredString {
    if points > 0 {
        text.green()
    } else if points < 0 {
        text.red()
    } else {
        text.normal()
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{}...", truncated.trim_end())
    } else {
        truncated
    }
}

fn summarize_list(values: &[String], limit: usize) -> String {
    let mut summary = values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    if values.len() > limit {
        summary.push_str(&format!(", ... {} more", values.len() - limit));
    }

    summary
}
