use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use glintindex_core::SearchQuery;
use glintindex_core::app::ApplicationService;

#[derive(Args)]
pub struct SearchArgs {
    /// The search query
    pub query: String,
}

pub fn execute(config_path: &str, args: SearchArgs) -> Result<()> {
    let service = ApplicationService::with_config_path(Path::new(config_path))
        .context("Failed to initialize application service. Check your configuration file.")?;

    let query = SearchQuery::new(&args.query);
    let results = service.search(&query).context("Search failed")?;

    if results.is_empty() {
        println!("No results found for: {}", args.query);
        return Ok(());
    }

    println!("{} results found\n", results.len());

    for (i, result) in results.iter().enumerate() {
        println!("{}. {}", i + 1, result.document.filename());
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

        if i < results.len() - 1 {
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
