//! Plain-text projection of an editor document.
//!
//! The phone used to render card previews in Kotlin, which meant one FFI call
//! and one JSON parse per note. The projection lives here so a single core call
//! can return a home screen's worth of previews, and so every client agrees on
//! what a note "says".
//!
//! The rules mirror the editor's: paragraphs, headings and code keep their text;
//! list items get a bullet; task items get their checked state; a horizontal
//! rule becomes `---`; images contribute nothing (the contract carries no alt
//! text); blank paragraphs are skipped.

use serde_json::Value;

fn text_of(node: &Value) -> String {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("text") => node
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => node
            .get("content")
            .and_then(|c| c.as_array())
            .map(|children| children.iter().map(text_of).collect::<String>())
            .unwrap_or_default(),
    }
}

fn walk(node: &Value, lines: &mut Vec<String>) {
    let kind = node.get("type").and_then(|t| t.as_str()).unwrap_or_default();
    match kind {
        "paragraph" | "heading" | "codeBlock" => {
            let text = text_of(node).trim().to_string();
            if !text.is_empty() {
                lines.push(text);
            }
        }
        "horizontalRule" => lines.push("---".to_string()),
        "taskItem" => {
            let checked = node
                .get("attrs")
                .and_then(|a| a.get("checked"))
                .and_then(|c| c.as_bool())
                .unwrap_or(false);
            let mark = if checked { "☑ " } else { "☐ " };
            let own: String = node
                .get("content")
                .and_then(|c| c.as_array())
                .map(|children| {
                    children
                        .iter()
                        .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("paragraph"))
                        .map(|c| text_of(c).trim().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            if !own.trim().is_empty() {
                lines.push(format!("{mark}{}", own.trim()));
            }
            for child in node.get("content").and_then(|c| c.as_array()).into_iter().flatten() {
                if child.get("type").and_then(|t| t.as_str()) != Some("paragraph") {
                    walk(child, lines);
                }
            }
        }
        "listItem" => {
            let own: String = node
                .get("content")
                .and_then(|c| c.as_array())
                .map(|children| {
                    children
                        .iter()
                        .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("paragraph"))
                        .map(|c| text_of(c).trim().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            if !own.trim().is_empty() {
                lines.push(format!("• {}", own.trim()));
            }
            for child in node.get("content").and_then(|c| c.as_array()).into_iter().flatten() {
                if child.get("type").and_then(|t| t.as_str()) != Some("paragraph") {
                    walk(child, lines);
                }
            }
        }
        _ => {
            for child in node.get("content").and_then(|c| c.as_array()).into_iter().flatten() {
                walk(child, lines);
            }
        }
    }
}

/// Flatten a document (or a bare array of nodes) into the text a card shows.
pub fn plain_text(json: &str) -> String {
    let value: Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(_) => return String::new(),
    };
    let mut lines = Vec::new();
    match &value {
        Value::Array(nodes) => nodes.iter().for_each(|n| walk(n, &mut lines)),
        _ => walk(&value, &mut lines),
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::plain_text;

    /// The same canonical document the core preserves verbatim and the editor
    /// round-trip test uses: every node kind the contract supports.
    fn fixture() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../core-db/tests/fixtures/block-schema-v1.json"
        ))
        .expect("golden fixture")
    }

    #[test]
    fn golden_fixture_flattens_with_its_markers() {
        let text = plain_text(&fixture());
        assert!(text.contains("Phase 0 editor slice"), "{text}");
        assert!(text.contains("Plain text with bold, italic, code and a link."), "{text}");
        assert!(text.contains("• First bullet"), "{text}");
        assert!(text.contains("• Second bullet"), "{text}");
        assert!(text.contains("• First step"), "{text}");
        assert!(text.contains("☐ Todo item"), "{text}");
        assert!(text.contains("☑ Done item"), "{text}");
        assert!(text.contains("---"), "{text}");
        assert!(!text.contains("image"), "{text}");
    }

    #[test]
    fn blank_paragraphs_never_produce_empty_lines() {
        let doc = r#"{"type":"doc","content":[
            {"type":"paragraph","content":[{"type":"text","text":"First"}]},
            {"type":"paragraph"},
            {"type":"paragraph","content":[{"type":"text","text":"Second"}]}
        ]}"#;
        assert_eq!(plain_text(doc), "First\nSecond");
    }

    #[test]
    fn a_nested_list_keeps_both_lines() {
        let doc = r#"{"type":"doc","content":[
            {"type":"bulletList","content":[
                {"type":"listItem","content":[
                    {"type":"paragraph","content":[{"type":"text","text":"Outer"}]},
                    {"type":"bulletList","content":[
                        {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Inner"}]}]}
                    ]}
                ]}
            ]}
        ]}"#;
        assert_eq!(plain_text(doc), "• Outer\n• Inner");
    }

    #[test]
    fn malformed_json_flattens_to_nothing() {
        assert_eq!(plain_text("not json"), "");
        assert_eq!(plain_text(""), "");
    }
}
