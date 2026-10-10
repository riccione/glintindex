//! Renders tantivy snippet HTML for terminal output.
//!
//! Tantivy wraps search matches in `<b>…</b>` tags inside an
//! HTML-escaped snippet. [`highlight_snippet`] converts that to plain
//! text with optional ANSI bold-yellow match highlighting.

/// ANSI escape: bold yellow.
const MATCH_ON: &str = "\x1b[1;33m";
/// ANSI escape: reset all attributes.
const RESET: &str = "\x1b[0m";

/// Minimal HTML entities emitted by tantivy's `encode_minimal`.
const ENTITIES: &[(&str, char)] = &[
    ("&amp;", '&'),
    ("&lt;", '<'),
    ("&gt;", '>'),
    ("&quot;", '"'),
    ("&#39;", '\''),
    ("&#x27;", '\''),
];

/// Converts a tantivy HTML snippet to terminal-ready text.
///
/// When `enabled` is true, text inside `<b>…</b>` match tags is wrapped
/// in bold-yellow ANSI escapes. HTML entities (`&amp;`, `&lt;`, `&gt;`,
/// `&quot;`, `&#39;`, `&#x27;`) are decoded; other tags are stripped.
/// The result is trimmed.
pub fn highlight_snippet(html: &str, enabled: bool) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_bold = false;
    let mut i = 0;

    while i < html.len() {
        let rest = &html[i..];
        let c = rest.chars().next().expect("index at char boundary");
        match c {
            '<' => {
                if let Some(end) = rest.find('>') {
                    let tag = rest[1..end].trim();
                    if tag == "b" {
                        if enabled && !in_bold {
                            out.push_str(MATCH_ON);
                        }
                        in_bold = true;
                    } else if tag == "/b" && in_bold {
                        if enabled {
                            out.push_str(RESET);
                        }
                        in_bold = false;
                    }
                    // All other tags are dropped.
                    i += end + 1;
                } else {
                    // Unterminated tag — emit '<' literally.
                    out.push(c);
                    i += c.len_utf8();
                }
            }
            '&' => {
                let decoded = ENTITIES
                    .iter()
                    .find(|(name, _)| rest.starts_with(*name))
                    .map(|&(name, decoded)| (name.len(), decoded));
                match decoded {
                    Some((len, decoded)) => {
                        out.push(decoded);
                        i += len;
                    }
                    None => {
                        out.push('&');
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(c);
                i += c.len_utf8();
            }
        }
    }

    if in_bold && enabled {
        out.push_str(RESET);
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_bold_content_with_ansi() {
        let out = highlight_snippet("hello <b>world</b>!", true);
        assert_eq!(out, "hello \x1b[1;33mworld\x1b[0m!");
    }

    #[test]
    fn plain_mode_strips_tags_without_ansi() {
        let out = highlight_snippet("hello <b>world</b>!", false);
        assert_eq!(out, "hello world!");
        assert!(!out.contains('\x1b'));
    }

    #[test]
    fn decodes_amp_entity() {
        let out = highlight_snippet("fish &amp; chips", false);
        assert_eq!(out, "fish & chips");
    }

    #[test]
    fn decodes_lt_gt_entities() {
        let out = highlight_snippet("a &lt;b&gt; c", false);
        assert_eq!(out, "a <b> c");
    }

    #[test]
    fn decodes_quot_and_apos_entities() {
        let out = highlight_snippet("&quot;don&#39;t&quot;", false);
        assert_eq!(out, "\"don't\"");
    }

    #[test]
    fn strips_unknown_tags() {
        let out = highlight_snippet("keep <em>this</em>", false);
        assert_eq!(out, "keep this");
    }

    #[test]
    fn trims_whitespace() {
        let out = highlight_snippet("  <b>match</b>  ", false);
        assert_eq!(out, "match");
    }

    #[test]
    fn multiple_matches_each_wrapped() {
        let out = highlight_snippet("<b>a</b> mid <b>b</b>", true);
        assert_eq!(out, "\x1b[1;33ma\x1b[0m mid \x1b[1;33mb\x1b[0m");
    }

    #[test]
    fn unterminated_bold_resets_at_end() {
        let out = highlight_snippet("start <b>never closed", true);
        assert!(out.contains(MATCH_ON));
        assert!(out.ends_with(RESET));
    }

    #[test]
    fn bare_ampersand_preserved() {
        let out = highlight_snippet("a & b", false);
        assert_eq!(out, "a & b");
    }

    #[test]
    fn empty_input() {
        assert_eq!(highlight_snippet("", true), "");
        assert_eq!(highlight_snippet("", false), "");
    }

    #[test]
    fn entity_inside_bold_decoded_and_wrapped() {
        let out = highlight_snippet("<b>a &amp; b</b>", true);
        assert_eq!(out, "\x1b[1;33ma & b\x1b[0m");
    }
}
