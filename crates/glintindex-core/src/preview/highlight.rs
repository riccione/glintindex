//! Search match highlighting for file previews.
//!
//! Highlights occurrences of search queries within preview text
//! while preserving syntax highlighting information.

use crate::preview::syntax::Style;

/// A highlighted match in the preview text.
#[derive(Debug, Clone)]
pub struct HighlightedMatch {
    /// The start byte offset of the match.
    pub start: usize,
    /// The end byte offset of the match.
    pub end: usize,
    /// The matched text.
    pub text: String,
}

/// Highlights search matches in a line of text.
///
/// Returns a list of match positions. The matches are case-insensitive.
pub fn find_matches(line: &str, query: &str) -> Vec<HighlightedMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let line_lower = line.to_lowercase();
    let mut matches = Vec::new();
    let mut start = 0;

    while let Some(pos) = line_lower[start..].find(&query_lower) {
        let absolute_pos = start + pos;
        let end = absolute_pos + query.len();
        matches.push(HighlightedMatch {
            start: absolute_pos,
            end,
            text: line[absolute_pos..end].to_string(),
        });
        // Advance by 1 to find overlapping matches
        start = absolute_pos + 1;
    }

    matches
}

/// Combines syntax highlighting with search match highlighting.
///
/// For each line, produces styled segments that account for both
/// syntax highlighting and search match highlighting.
pub fn combine_highlights(
    syntax_spans: &[(usize, usize, Style)],
    matches: &[HighlightedMatch],
    match_style: Style,
) -> Vec<(usize, usize, Style)> {
    let mut result = Vec::new();
    let mut syntax_idx = 0;
    let mut match_idx = 0;
    let mut pos = 0;

    let syntax_spans = syntax_spans.to_vec();
    let matches = matches.to_vec();

    loop {
        if syntax_idx >= syntax_spans.len() && match_idx >= matches.len() {
            break;
        }

        let syntax_end = syntax_spans
            .get(syntax_idx)
            .map(|s| s.1)
            .unwrap_or(usize::MAX);
        let match_start = matches
            .get(match_idx)
            .map(|m| m.start)
            .unwrap_or(usize::MAX);
        let match_end = matches.get(match_idx).map(|m| m.end).unwrap_or(usize::MAX);

        if pos >= syntax_end {
            syntax_idx += 1;
            continue;
        }

        if pos >= match_end {
            match_idx += 1;
            continue;
        }

        // Determine which span comes first
        if match_start <= pos && match_start < syntax_end {
            // We're in a match region
            let end = match_end.min(syntax_end);
            result.push((pos, end, match_style));
            pos = end;
            if pos >= match_end {
                match_idx += 1;
            }
        } else if pos < syntax_end {
            // We're in a syntax region
            let end = syntax_end.min(match_start);
            if let Some(span) = syntax_spans.get(syntax_idx) {
                result.push((pos, end, span.2));
            }
            pos = end;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_matches_basic() {
        let matches = find_matches("Hello world, hello!", "hello");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 5);
        assert_eq!(matches[1].start, 13);
        assert_eq!(matches[1].end, 18);
    }

    #[test]
    fn find_matches_case_insensitive() {
        let matches = find_matches("Hello HELLO hello", "hello");
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn find_matches_empty_query() {
        let matches = find_matches("Hello world", "");
        assert!(matches.is_empty());
    }

    #[test]
    fn find_matches_no_match() {
        let matches = find_matches("Hello world", "xyz");
        assert!(matches.is_empty());
    }

    #[test]
    fn find_matches_overlapping() {
        let matches = find_matches("aaa", "aa");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[1].start, 1);
    }

    #[test]
    fn combine_highlights_basic() {
        let syntax_spans = vec![(0, 5, Style::new((255, 0, 0), false, false))];
        let matches = vec![HighlightedMatch {
            start: 2,
            end: 4,
            text: "lo".to_string(),
        }];
        let match_style = Style::new((255, 255, 0), true, false);

        let result = combine_highlights(&syntax_spans, &matches, match_style);
        assert!(!result.is_empty());
    }

    #[test]
    fn combine_highlights_empty() {
        let result = combine_highlights(&[], &[], Style::default());
        assert!(result.is_empty());
    }

    #[test]
    fn match_text_preserved() {
        let matches = find_matches("test test test", "test");
        assert_eq!(matches[0].text, "test");
        assert_eq!(matches[1].text, "test");
        assert_eq!(matches[2].text, "test");
    }

    #[test]
    fn find_matches_at_boundaries() {
        let matches = find_matches("hello", "hello");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 5);
    }

    #[test]
    fn find_matches_special_chars() {
        let matches = find_matches("foo.bar.baz", ".");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn find_matches_unicode() {
        let matches = find_matches("café café", "café");
        assert_eq!(matches.len(), 2);
    }
}
