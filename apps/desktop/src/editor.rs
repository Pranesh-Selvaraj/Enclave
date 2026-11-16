//! Headless ProseMirror document model.
//!
//! This is the first slice of the in-house editor engine: a lossless JSON
//! representation of the contract in `docs/CONTRACT.md` §2. It does not edit
//! yet — the next slices add transactions, selection and history — but it
//! proves the boundary the native editor must honour: the canonical fixture
//! round-trips through Rust without a webview, and unknown attributes survive.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A ProseMirror/TipTap document root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProseMirrorDoc {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub content: Vec<ProseMirrorNode>,
}

/// One block or inline node. Unknown node types and attributes are preserved
/// verbatim: the contract allows additive nodes, and older editors must be able
/// to carry a newer document without dropping data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProseMirrorNode {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ProseMirrorNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marks: Option<Vec<ProseMirrorMark>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProseMirrorMark {
    #[serde(rename = "type")]
    pub mark_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocError {
    InvalidJson(String),
    NotADocument,
    TooDeep,
    TooLarge,
    TextAndContent,
}

impl std::fmt::Display for DocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid document JSON: {message}"),
            Self::NotADocument => write!(f, "the root node must be a doc"),
            Self::TooDeep => write!(f, "the document is nested too deeply"),
            Self::TooLarge => write!(f, "the document has too many nodes"),
            Self::TextAndContent => write!(f, "a text node cannot have child content"),
        }
    }
}

impl std::error::Error for DocError {}

impl ProseMirrorDoc {
    pub fn from_json_str(json: &str) -> Result<Self, DocError> {
        let doc: Self =
            serde_json::from_str(json).map_err(|error| DocError::InvalidJson(error.to_string()))?;
        doc.validate()?;
        Ok(doc)
    }

    #[allow(dead_code)] // the save path lands with the view engine; tests use it today
    pub fn to_json_string(&self) -> String {
        // Serialisation cannot fail for this owned tree.
        serde_json::to_string(self).expect("serialise a ProseMirror document")
    }

    #[allow(dead_code)] // used by the fixture tests and the upcoming view engine
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("serialise a ProseMirror document")
    }

    /// A plain-text projection for the notes list and previews. This is not
    /// the editor's layout; it is only enough to show a note before the view
    /// engine exists.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for node in &self.content {
            append_plain_text(node, &mut out);
        }
        out.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Bounds for untrusted content. The canonical fixture is far below these;
    /// the limits keep a malformed or hostile vault from exhausting the UI.
    pub fn validate(&self) -> Result<(), DocError> {
        if self.node_type != "doc" {
            return Err(DocError::NotADocument);
        }
        let mut nodes = 0usize;
        for node in &self.content {
            validate_node(node, 0, &mut nodes)?;
        }
        Ok(())
    }
}

const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 100_000;

fn append_plain_text(node: &ProseMirrorNode, out: &mut String) {
    if let Some(text) = &node.text {
        out.push_str(text);
    }
    if node.node_type == "hardBreak" {
        out.push('\n');
    }
    if let Some(children) = &node.content {
        for child in children {
            append_plain_text(child, out);
        }
    }
    if matches!(
        node.node_type.as_str(),
        "paragraph"
            | "heading"
            | "listItem"
            | "taskItem"
            | "blockquote"
            | "codeBlock"
            | "tableRow"
            | "horizontalRule"
    ) {
        out.push('\n');
    }
}

fn validate_node(node: &ProseMirrorNode, depth: usize, nodes: &mut usize) -> Result<(), DocError> {
    if depth > MAX_DEPTH {
        return Err(DocError::TooDeep);
    }
    *nodes += 1;
    if *nodes > MAX_NODES {
        return Err(DocError::TooLarge);
    }
    if node.text.is_some()
        && node
            .content
            .as_ref()
            .is_some_and(|content| !content.is_empty())
    {
        return Err(DocError::TextAndContent);
    }
    if let Some(children) = &node.content {
        for child in children {
            validate_node(child, depth + 1, nodes)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL_FIXTURE: &str =
        include_str!("../../../core/core-db/tests/fixtures/block-schema-v1.json");

    #[test]
    fn canonical_fixture_round_trips_semantically() {
        let doc =
            ProseMirrorDoc::from_json_str(CANONICAL_FIXTURE).expect("canonical fixture parses");
        let expected: Value = serde_json::from_str(CANONICAL_FIXTURE).expect("fixture is JSON");
        assert_eq!(doc.to_value(), expected);
    }

    #[test]
    fn unknown_attributes_and_node_types_survive() {
        let json = r#"{
            "type": "doc",
            "content": [
                {"type": "heading", "attrs": {"level": 2, "vendorFlag": true},
                 "content": [{"type": "text", "text": "Hello"}]},
                {"type": "futureBlock", "attrs": {"payload": {"a": [1, 2]}}}
            ]
        }"#;
        let doc = ProseMirrorDoc::from_json_str(json).expect("future node is accepted");
        let expected: Value = serde_json::from_str(json).expect("fixture is JSON");
        assert_eq!(doc.to_value(), expected);
    }

    #[test]
    fn plain_text_projection_reads_the_fixture() {
        let doc =
            ProseMirrorDoc::from_json_str(CANONICAL_FIXTURE).expect("canonical fixture parses");
        let text = doc.plain_text();
        assert!(text.contains("Phase 0 editor slice"));
        assert!(text.contains("First bullet"));
        assert!(text.contains("Todo item"));
    }

    #[test]
    fn rejects_non_documents_and_text_with_children() {
        assert_eq!(
            ProseMirrorDoc::from_json_str(r#"{"type":"paragraph","content":[]}"#),
            Err(DocError::NotADocument)
        );
        assert_eq!(
            ProseMirrorDoc::from_json_str(
                r#"{"type":"doc","content":[{"type":"text","text":"x","content":[{"type":"text","text":"y"}]}]}"#
            ),
            Err(DocError::TextAndContent)
        );
    }
}
