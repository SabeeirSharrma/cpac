use colored::{ColoredString, Colorize};
use std::collections::BTreeSet;

use crate::{
    audit::SystemAudit,
    backends::{PackageInfo, PackageSource},
    trust::TrustReport,
};

const DEFAULT_MAX_RESULTS: usize = 25;

pub fn print_search_results(results: &[PackageInfo], show_all: bool) {
    if results.is_empty() {
        println!("{}", "No packages found.".dimmed());
        return;
    }

    println!(
        "{:<32} {:<14} {:<16} {}",
        "Package".cyan().bold(),
        "Version".cyan().bold(),
        "Source".cyan().bold(),
        "Description".cyan().bold()
    );

    let limit = if show_all {
        results.len()
    } else {
        DEFAULT_MAX_RESULTS
    };

    for pkg in results.iter().take(limit) {
        println!(
            "{:<32} {:<14} {:<16} {}",
            pkg.name.as_str().bold(),
            truncate(&pkg.version, 14),
            source_badge(&pkg.source),
            truncate(&pkg.description, 80)
        );
    }

    if !show_all && results.len() > DEFAULT_MAX_RESULTS {
        println!(
            "{}",
            format!(
                "\nShowing {} of {} results. Use --all to view everything.",
                DEFAULT_MAX_RESULTS,
                results.len()
            )
            .dimmed()
        );
    }
}

pub fn print_trust_report(pkg: &PackageInfo, report: &TrustReport) {
    println!();
    println!(
        "  {} {}",
        "Package:".cyan().bold(),
        report.package_name.as_str().bold()
    );
    println!("  {} {}", "Version:".cyan().bold(), pkg.version);
    println!("  {} {}", "Repository:".cyan().bold(), pkg.source);
    println!("  {} {}", "Trust Tier:".cyan().bold(), report.tier);

    match &pkg.maintainer {
        Some(maintainer) => println!("  {} {}", "Maintainer:".cyan().bold(), maintainer),
        None => println!("  {} {}", "Maintainer:".cyan().bold(), "Unknown".dimmed()),
    }

    match &pkg.url {
        Some(url) => println!("  {} {}", "URL:".cyan().bold(), url),
        None => println!("  {} {}", "URL:".cyan().bold(), "Unknown".dimmed()),
    }

    match pkg.popularity {
        Some(popularity) => println!("  {} {:.2}", "Popularity:".cyan().bold(), popularity),
        None => println!(
            "  {} {}  {}",
            "Popularity:".cyan().bold(),
            "Unknown".dimmed(),
            "(Reason: Metadata unavailable)".dimmed()
        ),
    }

    match &pkg.install_size {
        Some(size) => println!("  {} {}", "Installed Size:".cyan().bold(), size),
        None => println!(
            "  {} {}  {}",
            "Installed Size:".cyan().bold(),
            "Unknown".dimmed(),
            "(Reason: Metadata unavailable)".dimmed()
        ),
    }

    if !pkg.licenses.is_empty() {
        println!(
            "  {} {}",
            "Licenses:".cyan().bold(),
            pkg.licenses.join(", ")
        );
    } else {
        println!(
            "  {} {}  {}",
            "Licenses:".cyan().bold(),
            "Unknown".dimmed(),
            "(Reason: Metadata unavailable)".dimmed()
        );
    }

    if !pkg.depends.is_empty() {
        println!(
            "  {} {}",
            "Dependencies:".cyan().bold(),
            summarize_list(&pkg.depends, 8)
        );
    }

    if !pkg.description.is_empty() {
        println!("  {} {}", "Description:".cyan().bold(), pkg.description);
    }

    // --- Trust Score Box ---
    println!();
    let box_border = "══════════════════════════════════════";
    let score_color = score_tier_color(report.score);

    println!("  {}", colorize_str(box_border, score_color));
    println!(
        "  {} {}",
        colorize_str("Trust Score:", score_color).bold(),
        colored_score(report.score).bold()
    );
    println!(
        "  {} {}",
        colorize_str("Recommendation:", score_color).bold(),
        colored_recommendation(&report.recommendation)
    );
    println!("  {}", colorize_str(box_border, score_color));

    // --- Signal Breakdown ---
    println!();
    println!("  {}", "Signals".cyan().bold());
    for signal in &report.signals {
        let points = if signal.points >= 0 {
            format!("+{}/{}", signal.points, signal.max_points)
        } else {
            signal.points.to_string()
        };

        println!(
            "    {:<20} {:>7}  {}",
            signal.name.as_str().bold(),
            color_points(signal.points, &points),
            signal.detail
        );
    }
    println!();
}

pub fn print_system_audit(audit: &SystemAudit) {
    println!();
    println!("  {}", "System Audit".cyan().bold());
    println!(
        "  {} {}",
        "Installed Packages:".cyan().bold(),
        audit.counts.total()
    );
    println!("  {} {}", "Official:".cyan().bold(), audit.counts.official);
    println!(
        "  {} {}",
        "Third Party:".cyan().bold(),
        audit.counts.third_party
    );
    println!(
        "  {} {}",
        "Community:".cyan().bold(),
        audit.counts.community
    );
    println!("  {} {}", "Unknown:".cyan().bold(), audit.counts.unknown);

    if !audit.official_notices.is_empty() {
        let repos = audit
            .official_notices
            .iter()
            .map(|notice| official_repo_label(&notice.repo).to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        println!();
        println!(
            "  {} {} package(s) from official repositories ({}) are excluded from warnings",
            "Official packages:".cyan().bold(),
            audit.official_notices.len(),
            repos
        );
    }

    println!();
    println!("  {}", "Warnings".cyan().bold());

    if audit.warnings.is_empty() {
        println!("    {}", "No warnings found.".dimmed());
        println!();
        return;
    }

    for warning in &audit.warnings {
        println!(
            "    {:<24} [Trust: {} - {}]",
            warning.package_name.as_str().bold(),
            colored_warning_label(warning.score, &warning.tier),
            warning.reason
        );
    }
    println!();
}

/// Color tier for the score box.
#[derive(Clone, Copy)]
enum ScoreColor {
    Green,
    Yellow,
    Red,
}

fn score_tier_color(score: u32) -> ScoreColor {
    match score {
        70..=100 => ScoreColor::Green,
        40..=69 => ScoreColor::Yellow,
        _ => ScoreColor::Red,
    }
}

fn colorize_str(text: &str, color: ScoreColor) -> ColoredString {
    match color {
        ScoreColor::Green => text.green(),
        ScoreColor::Yellow => text.yellow(),
        ScoreColor::Red => text.red(),
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

fn colored_warning_label(score: u32, tier: &crate::trust::TrustTier) -> ColoredString {
    if matches!(tier, crate::trust::TrustTier::Unknown) {
        "Unknown".dimmed()
    } else {
        colored_score(score)
    }
}

fn official_repo_label(repo: &str) -> ColoredString {
    if repo == "endeavouros" {
        "EndeavourOS".green()
    } else if repo.starts_with("cachyos") {
        "CachyOS".green()
    } else {
        repo.green()
    }
}

fn colored_recommendation(recommendation: &str) -> ColoredString {
    let upper = recommendation.to_uppercase();
    match recommendation {
        "Safe" | "Moderate" => upper.green().bold(),
        "Caution" => upper.yellow().bold(),
        _ => upper.red().bold(),
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
