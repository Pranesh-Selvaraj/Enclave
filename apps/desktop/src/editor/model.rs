//! Headless ProseMirror document model.
//!
//! This is the contract boundary the editor engine builds on. It is lossless:
//! unknown node types and attributes survive a load/save cycle, so a newer
//! document can pass through an older editor without being rewritten. Editing
//! operations live in [`crate::editor::engine`].

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A path from the document root's `content` to one node. An empty path is the
/// document itself; `[3, 0, 0]` is the first paragraph inside the first list
/// item of the fourth top-level block.
pub type NodePath = Vec<usize>;

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

    pub fn to_json_string(&self) -> String {
        // Serialisation cannot fail for this owned tree.
        serde_json::to_string(self).expect("serialise a ProseMirror document")
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("serialise a ProseMirror document")
    }

    /// Resolve a path relative to the root's `content`.
    pub fn node(&self, path: &[usize]) -> Option<&ProseMirrorNode> {
        find_node(&self.content, path)
    }

    pub fn node_mut(&mut self, path: &[usize]) -> Option<&mut ProseMirrorNode> {
        find_node_mut(&mut self.content, path)
    }

    /// Insert `node` after the node at `path`, in the same parent.
    pub fn insert_sibling(&mut self, path: &[usize], node: ProseMirrorNode) -> bool {
        let Some((last, parent)) = path.split_last() else {
            return false;
        };
        let Some(content) = self.content_mut_at(parent) else {
            return false;
        };
        if *last > content.len() {
            return false;
        }
        content.insert(last + 1, node);
        true
    }

    /// Insert `node` at `path`, replacing the slot at that index by shifting
    /// the current node and its siblings right.
    pub fn insert_node(&mut self, path: &[usize], node: ProseMirrorNode) -> bool {
        let Some((last, parent)) = path.split_last() else {
            return false;
        };
        let Some(content) = self.content_mut_at(parent) else {
            return false;
        };
        if *last > content.len() {
            return false;
        }
        content.insert(*last, node);
        true
    }

    /// Remove and return the node at `path`.
    pub fn remove_node(&mut self, path: &[usize]) -> Option<ProseMirrorNode> {
        let (last, parent) = path.split_last()?;
        let content = self.content_mut_at(parent)?;
        (last < &content.len()).then(|| content.remove(*last))
    }

    /// The mutable child list of the node at `path`; empty path is the root.
    pub fn content_mut_at(&mut self, path: &[usize]) -> Option<&mut Vec<ProseMirrorNode>> {
        if path.is_empty() {
            return Some(&mut self.content);
        }
        self.node_mut(path)?.content.as_mut()
    }

    /// A plain-text projection for previews and tests.
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

impl ProseMirrorNode {
    pub fn text_node(text: impl Into<String>, marks: Option<Vec<ProseMirrorMark>>) -> Self {
        let text = text.into();
        Self {
            node_type: "text".to_owned(),
            attrs: None,
            content: None,
            marks,
            text: Some(text),
        }
    }

    pub fn paragraph(content: Vec<ProseMirrorNode>) -> Self {
        Self {
            node_type: "paragraph".to_owned(),
            attrs: None,
            content: (!content.is_empty()).then_some(content),
            marks: None,
            text: None,
        }
    }

    pub fn is_text_block(&self) -> bool {
        matches!(
            self.node_type.as_str(),
            "paragraph" | "heading" | "codeBlock"
        )
    }

    pub fn children(&self) -> &[ProseMirrorNode] {
        self.content.as_deref().unwrap_or(&[])
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<ProseMirrorNode>> {
        self.content.as_mut()
    }

    pub fn inline_text(&self) -> String {
        let mut out = String::new();
        for child in self.children() {
            if let Some(text) = &child.text {
                out.push_str(text);
            } else if child.node_type == "hardBreak" {
                out.push('\n');
            }
        }
        out
    }
}

impl ProseMirrorMark {
    pub fn new(mark_type: impl Into<String>) -> Self {
        Self {
            mark_type: mark_type.into(),
            attrs: None,
        }
    }

    pub fn with_attrs(mark_type: impl Into<String>, attrs: Value) -> Self {
        Self {
            mark_type: mark_type.into(),
            attrs: Some(attrs),
        }
    }
}

fn find_node<'a>(nodes: &'a [ProseMirrorNode], path: &[usize]) -> Option<&'a ProseMirrorNode> {
    let (first, rest) = path.split_first()?;
    let node = nodes.get(*first)?;
    if rest.is_empty() {
        Some(node)
    } else {
        find_node(node.content.as_deref()?, rest)
    }
}

fn find_node_mut<'a>(
    nodes: &'a mut [ProseMirrorNode],
    path: &[usize],
) -> Option<&'a mut ProseMirrorNode> {
    let (first, rest) = path.split_first()?;
    let node = nodes.get_mut(*first)?;
    if rest.is_empty() {
        Some(node)
    } else {
        find_node_mut(node.content.as_mut()?.as_mut_slice(), rest)
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
        include_str!("../../../../core/core-db/tests/fixtures/block-schema-v1.json");

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
    fn resolves_and_mutates_paths() {
        let mut doc = ProseMirrorDoc::from_json_str(
            r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"a"}]},{"type":"paragraph","content":[{"type":"text","text":"b"}]}]}"#,
        )
        .expect("doc");
        assert_eq!(
            doc.node(&[0]).map(|node| node.node_type.as_str()),
            Some("paragraph")
        );
        doc.node_mut(&[0]).expect("paragraph").node_type = "heading".to_owned();
        assert_eq!(
            doc.node(&[0]).map(|node| node.node_type.as_str()),
            Some("heading")
        );
        assert!(doc.insert_sibling(
            &[0],
            ProseMirrorNode::paragraph(vec![ProseMirrorNode::text_node("middle", None)])
        ));
        assert_eq!(doc.content.len(), 3);
        assert_eq!(
            doc.remove_node(&[1]).map(|node| node.inline_text()),
            Some("middle".to_owned())
        );
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
