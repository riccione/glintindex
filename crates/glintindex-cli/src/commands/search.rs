use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use glintindex_core::SearchQuery;
use glintindex_core::app::ApplicationService;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query string
    pub query: String,

    /// Page number to display (1-based index)
    #[arg(short = 'p', long = "page", default_value_t = 1)]
    pub page: usize,

    /// Maximum number of results per page (overrides config default)
    #[arg(short = 'l', long = "limit")]
    pub limit: Option<usize>,
}

pub fn execute(config_path: &str, args: SearchArgs) -> Result<()> {
    let service = ApplicationService::with_config_path(Path::new(config_path))
        .context("Failed to initialize application service. Check your configuration file.")?;

    let effective_limit = args
        .limit
        .unwrap_or(service.config().pagination.default_page_size);
    let query = SearchQuery::paged(&args.query, args.page, effective_limit);
    let response = service.search(&query).context("Search failed")?;

    if response.is_empty() {
        println!("No results found for: {}", args.query);
        return Ok(());
    }

    // Calculate human-readable range bounds
    let start = if response.total == 0 {
        0
    } else {
        response.offset + 1
    };
    let end = (response.offset + response.results.len()).min(response.total);
    let current_page = response.current_page();
    let total_pages = response.total_pages();

    println!(
        "Showing {}–{} of {} results (Page {} of {})\n",
        start, end, response.total, current_page, total_pages
    );
    println!("{}", "─".repeat(80));

    for (i, result) in response.results.iter().enumerate() {
        println!("{}. {}", start + i, result.document.filename());
        println!();
        println!("{}", result.document.path.display());

        if !result.snippet.is_empty() {
            println!();
            // Strip HTML tags from snippet for plain text output
            let plain_snippet = strip_html_tags(&result.snippet);
            if !plain_snippet.is_empty() {
                println!("{}", plain_snippet);
            }
        }

        if i < response.results.len() - 1 {
            println!();
        }
    }

    Ok(())
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_tags_removes_tags() {
        let html = "<em>hello</em> world";
        assert_eq!(strip_html_tags(html), "hello world");
    }

    #[test]
    fn strip_html_tags_plain_text() {
        let text = "no tags here";
        assert_eq!(strip_html_tags(text), "no tags here");
    }

    #[test]
    fn strip_html_tags_nested() {
        let html = "<p><strong>test</strong></p>";
        assert_eq!(strip_html_tags(html), "test");
    }
}
