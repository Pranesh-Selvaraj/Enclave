//! Headless editing engine for the ProseMirror block model.
//!
//! This slice supports single-block text editing, block splitting and merging,
//! mark toggling, task toggling and undo/redo. It is deliberately independent of
//! egui: the same commands are used by the view, tests and (later) the AI tools.

use serde_json::{json, Value};

use crate::editor::model::{DocError, NodePath, ProseMirrorDoc, ProseMirrorMark, ProseMirrorNode};
use crate::editor::selection::Selection;

const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone)]
struct Snapshot {
    doc: ProseMirrorDoc,
    selection: Selection,
}

pub struct Editor {
    doc: ProseMirrorDoc,
    selection: Selection,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl Editor {
    /// Load a document and put the caret in the first text block.
    pub fn new(mut doc: ProseMirrorDoc) -> Self {
        if doc.content.is_empty() {
            doc.content.push(ProseMirrorNode::paragraph(Vec::new()));
        }
        let selection = first_text_block_path(&doc.content)
            .map(|path| Selection::caret(path, 0))
            .unwrap_or_else(|| Selection::caret(vec![0], 0));
        Self {
            doc,
            selection,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn from_json_str(json: &str) -> Result<Self, DocError> {
        Ok(Self::new(ProseMirrorDoc::from_json_str(json)?))
    }

    pub fn doc(&self) -> &ProseMirrorDoc {
        &self.doc
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
        self.clamp_selection();
    }

    pub fn to_json_string(&self) -> String {
        self.doc.to_json_string()
    }

    pub fn to_value(&self) -> Value {
        self.doc.to_value()
    }

    pub fn selected_text_len(&self) -> usize {
        self.doc
            .node(&self.selection.path)
            .map(flatten_inline)
            .map(|pieces| pieces.iter().map(piece_len).sum())
            .unwrap_or(0)
    }

    pub fn insert_text(&mut self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let before = self.snapshot();
        if self.insert_text_inner(text) {
            self.finish_edit(before);
            true
        } else {
            false
        }
    }

    /// Replace a range in one text block. This is the view's diff path: the
    /// text editor reports a new string, the engine replaces only the changed
    /// span, and marks outside the span survive.
    pub fn replace_range(&mut self, path: NodePath, start: usize, end: usize, text: &str) -> bool {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let length = self
            .doc
            .node(&path)
            .map(flatten_inline)
            .map(|pieces| pieces.iter().map(piece_len).sum())
            .unwrap_or(0);
        let start = start.min(length);
        let end = end.min(length);
        let before = self.snapshot();
        self.selection = Selection::range(path, start, end);
        if self.insert_text_inner(text) {
            self.finish_edit(before);
            true
        } else {
            false
        }
    }

    pub fn delete_backward(&mut self) -> bool {
        let before = self.snapshot();
        if self.delete_backward_inner() {
            self.finish_edit(before);
            true
        } else {
            false
        }
    }

    pub fn delete_forward(&mut self) -> bool {
        let before = self.snapshot();
        if self.delete_forward_inner() {
            self.finish_edit(before);
            true
        } else {
            false
        }
    }

    /// Split at the caret. Headings continue as a paragraph; code blocks stay
    /// code blocks; paragraphs stay paragraphs.
    pub fn split_block(&mut self) -> bool {
        let before = self.snapshot();
        if self.split_block_inner() {
            self.finish_edit(before);
            true
        } else {
            false
        }
    }

    pub fn set_heading_level(&mut self, level: u8) -> bool {
        let before = self.snapshot();
        let Some(block) = self.doc.node_mut(&self.selection.path) else {
            return false;
        };
        if !block.is_text_block() {
            return false;
        }
        if level == 0 {
            block.node_type = "paragraph".to_owned();
            block.attrs = None;
        } else {
            if !(1..=6).contains(&level) {
                return false;
            }
            block.node_type = "heading".to_owned();
            let attrs = block.attrs.get_or_insert_with(|| json!({}));
            if let Value::Object(map) = attrs {
                map.insert("level".to_owned(), Value::from(level));
            } else {
                block.attrs = Some(json!({ "level": level }));
            }
        }
        self.finish_edit(before);
        true
    }

    pub fn set_block_type(&mut self, node_type: &str) -> bool {
        if !matches!(node_type, "paragraph" | "codeBlock" | "heading") {
            return false;
        }
        if node_type == "heading" {
            return self.set_heading_level(1);
        }
        let before = self.snapshot();
        let Some(block) = self.doc.node_mut(&self.selection.path) else {
            return false;
        };
        if !block.is_text_block() {
            return false;
        }
        block.node_type = node_type.to_owned();
        block.attrs = None;
        self.finish_edit(before);
        true
    }

    /// Toggle a mark over the selection. A caret is a no-op until pending marks
    /// are introduced with the view.
    pub fn toggle_mark(&mut self, mark: ProseMirrorMark) -> bool {
        let before = self.snapshot();
        let (start, end) = self.selection.range_offsets();
        if start == end {
            return false;
        }
        let path = self.selection.path.clone();
        let Some(block) = self.doc.node_mut(&path) else {
            return false;
        };
        if !block.is_text_block() {
            return false;
        }
        let mut pieces = flatten_inline(block);
        if !apply_mark(&mut pieces, start, end, &mark) {
            return false;
        }
        rebuild_inline(block, pieces);
        self.finish_edit(before);
        true
    }

    /// Toggle a `taskItem` ancestor's checked state.
    pub fn toggle_task(&mut self) -> bool {
        let before = self.snapshot();
        let path = self.selection.path.clone();
        for depth in (0..path.len()).rev() {
            let prefix = &path[..=depth];
            if self
                .doc
                .node(prefix)
                .is_some_and(|node| node.node_type == "taskItem")
            {
                let node = self
                    .doc
                    .node_mut(prefix)
                    .expect("the task item was just resolved");
                let attrs = node.attrs.get_or_insert_with(|| json!({}));
                if let Value::Object(map) = attrs {
                    let checked = map.get("checked").and_then(Value::as_bool).unwrap_or(false);
                    map.insert("checked".to_owned(), Value::Bool(!checked));
                } else {
                    node.attrs = Some(json!({ "checked": true }));
                }
                self.finish_edit(before);
                return true;
            }
        }
        false
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.redo.push(current);
        self.restore(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.undo.push(current);
        self.restore(next);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            doc: self.doc.clone(),
            selection: self.selection.clone(),
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.doc = snapshot.doc;
        self.selection = snapshot.selection;
    }

    fn finish_edit(&mut self, before: Snapshot) {
        self.undo.push(before);
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let len = self.selected_text_len();
        self.selection.clamp(len);
    }

    fn insert_text_inner(&mut self, text: &str) -> bool {
        let (start, end) = self.selection.range_offsets();
        let path = self.selection.path.clone();
        let Some(block) = self.doc.node_mut(&path) else {
            return false;
        };
        if !block.is_text_block() {
            return false;
        }
        let mut pieces = flatten_inline(block);
        let inherited = if start != end {
            marks_at(&pieces, start)
        } else {
            None
        };
        if start != end && !delete_range(&mut pieces, start, end) {
            return false;
        }
        insert_plain(&mut pieces, start, text, inherited);
        rebuild_inline(block, pieces);
        let length = text.chars().count();
        self.selection.anchor = start + length;
        self.selection.head = start + length;
        true
    }

    fn delete_backward_inner(&mut self) -> bool {
        let (start, end) = self.selection.range_offsets();
        if start != end {
            if !self.delete_range_inner(start, end) {
                return false;
            }
            self.selection.anchor = start;
            self.selection.head = start;
            return true;
        }
        if self.selection.head > 0 {
            let end = self.selection.head;
            if !self.delete_range_inner(end - 1, end) {
                return false;
            }
            self.selection.anchor = end - 1;
            self.selection.head = end - 1;
            return true;
        }
        self.merge_previous()
    }

    fn delete_forward_inner(&mut self) -> bool {
        let (start, end) = self.selection.range_offsets();
        if start != end {
            if !self.delete_range_inner(start, end) {
                return false;
            }
            self.selection.anchor = start;
            self.selection.head = start;
            return true;
        }
        let length = self.selected_text_len();
        if self.selection.head < length {
            let start = self.selection.head;
            return self.delete_range_inner(start, start + 1);
        }
        self.merge_next()
    }

    fn delete_range_inner(&mut self, start: usize, end: usize) -> bool {
        if start >= end {
            return false;
        }
        let path = self.selection.path.clone();
        let Some(block) = self.doc.node_mut(&path) else {
            return false;
        };
        if !block.is_text_block() {
            return false;
        }
        let mut pieces = flatten_inline(block);
        if !delete_range(&mut pieces, start, end) {
            return false;
        }
        rebuild_inline(block, pieces);
        true
    }

    fn split_block_inner(&mut self) -> bool {
        let (start, end) = self.selection.range_offsets();
        if start != end {
            if !self.delete_range_inner(start, end) {
                return false;
            }
            self.selection.anchor = start;
            self.selection.head = start;
        }
        let path = self.selection.path.clone();
        let Some(block) = self.doc.node(&path).cloned() else {
            return false;
        };
        if !block.is_text_block() {
            return false;
        }
        let offset = self.selection.head;
        let pieces = flatten_inline(&block);
        let (before, after) = split_pieces(pieces, offset);
        let mut first = block.clone();
        rebuild_inline(&mut first, before);
        let mut second = block;
        if second.node_type == "heading" {
            second.node_type = "paragraph".to_owned();
            second.attrs = None;
        }
        rebuild_inline(&mut second, after);
        let Some(first_node) = self.doc.node_mut(&path) else {
            return false;
        };
        *first_node = first;
        if !self.doc.insert_sibling(&path, second) {
            return false;
        }
        let mut next_path = path;
        if let Some(last) = next_path.last_mut() {
            *last += 1;
        }
        self.selection = Selection::caret(next_path, 0);
        true
    }

    fn merge_previous(&mut self) -> bool {
        let path = self.selection.path.clone();
        let Some(previous_path) = sibling_path(&path, -1) else {
            return false;
        };
        let Some(previous) = self.doc.node(&previous_path).cloned() else {
            return false;
        };
        let Some(current) = self.doc.node(&path).cloned() else {
            return false;
        };
        if !previous.is_text_block() || !current.is_text_block() {
            return false;
        }
        let previous_len = flatten_inline(&previous)
            .iter()
            .map(piece_len)
            .sum::<usize>();
        let mut merged = flatten_inline(&previous);
        merged.extend(flatten_inline(&current));
        let Some(target) = self.doc.node_mut(&previous_path) else {
            return false;
        };
        rebuild_inline(target, merged);
        self.doc.remove_node(&path);
        self.selection.path = previous_path;
        self.selection.anchor = previous_len;
        self.selection.head = previous_len;
        true
    }

    fn merge_next(&mut self) -> bool {
        let path = self.selection.path.clone();
        let Some(next_path) = sibling_path(&path, 1) else {
            return false;
        };
        let Some(next) = self.doc.node(&next_path).cloned() else {
            return false;
        };
        let Some(current) = self.doc.node(&path).cloned() else {
            return false;
        };
        if !next.is_text_block() || !current.is_text_block() {
            return false;
        }
        let mut merged = flatten_inline(&current);
        merged.extend(flatten_inline(&next));
        let Some(target) = self.doc.node_mut(&path) else {
            return false;
        };
        rebuild_inline(target, merged);
        self.doc.remove_node(&next_path);
        true
    }
}

fn first_text_block_path(nodes: &[ProseMirrorNode]) -> Option<NodePath> {
    fn walk(nodes: &[ProseMirrorNode], prefix: &mut NodePath) -> Option<NodePath> {
        for (index, node) in nodes.iter().enumerate() {
            prefix.push(index);
            if node.is_text_block() {
                return Some(prefix.clone());
            }
            if let Some(children) = node.content.as_deref() {
                if let Some(path) = walk(children, prefix) {
                    return Some(path);
                }
            }
            prefix.pop();
        }
        None
    }
    walk(nodes, &mut Vec::new())
}

fn sibling_path(path: &[usize], delta: isize) -> Option<NodePath> {
    let (last, parent) = path.split_last()?;
    let index = (*last as isize).checked_add(delta)?;
    if index < 0 {
        return None;
    }
    let mut sibling = parent.to_vec();
    sibling.push(index as usize);
    Some(sibling)
}

#[derive(Debug, Clone, PartialEq)]
enum InlinePiece {
    Text {
        text: String,
        marks: Option<Vec<ProseMirrorMark>>,
    },
    Atom(ProseMirrorNode),
}

fn flatten_inline(block: &ProseMirrorNode) -> Vec<InlinePiece> {
    if let Some(text) = &block.text {
        return if text.is_empty() {
            Vec::new()
        } else {
            vec![InlinePiece::Text {
                text: text.clone(),
                marks: normalize_marks(block.marks.clone()),
            }]
        };
    }
    let mut pieces = Vec::new();
    for child in block.children() {
        if let Some(text) = &child.text {
            if !text.is_empty() {
                pieces.push(InlinePiece::Text {
                    text: text.clone(),
                    marks: normalize_marks(child.marks.clone()),
                });
            }
        } else {
            pieces.push(InlinePiece::Atom(child.clone()));
        }
    }
    pieces
}

fn rebuild_inline(block: &mut ProseMirrorNode, pieces: Vec<InlinePiece>) {
    let mut nodes: Vec<ProseMirrorNode> = Vec::new();
    for piece in pieces {
        match piece {
            InlinePiece::Text { text, marks } if !text.is_empty() => {
                let marks = normalize_marks(marks);
                if let Some(last) = nodes.last_mut() {
                    if last.text.is_some() && last.marks == marks {
                        if let Some(last_text) = &mut last.text {
                            last_text.push_str(&text);
                        }
                        continue;
                    }
                }
                nodes.push(ProseMirrorNode::text_node(text, marks));
            }
            InlinePiece::Text { .. } => {}
            InlinePiece::Atom(node) => nodes.push(node),
        }
    }
    block.content = if nodes.is_empty() { None } else { Some(nodes) };
}

fn piece_len(piece: &InlinePiece) -> usize {
    match piece {
        InlinePiece::Text { text, .. } => text.chars().count(),
        InlinePiece::Atom(_) => 1,
    }
}

fn split_text(text: &str, local: usize) -> (String, String) {
    let byte = text
        .char_indices()
        .nth(local)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    (text[..byte].to_owned(), text[byte..].to_owned())
}

/// Split a text piece at `offset` so every remaining piece is wholly before or
/// wholly after the offset.
fn split_pieces_at(pieces: &mut Vec<InlinePiece>, offset: usize) {
    let mut cursor = 0;
    for index in 0..pieces.len() {
        let length = piece_len(&pieces[index]);
        if offset > cursor && offset < cursor + length {
            if let InlinePiece::Text { text, marks } = &pieces[index] {
                let (before, after) = split_text(text, offset - cursor);
                let marks = marks.clone();
                pieces.splice(
                    index..=index,
                    [
                        InlinePiece::Text {
                            text: before,
                            marks: marks.clone(),
                        },
                        InlinePiece::Text { text: after, marks },
                    ],
                );
            }
            return;
        }
        cursor += length;
    }
}

fn marks_at(pieces: &[InlinePiece], offset: usize) -> Option<Vec<ProseMirrorMark>> {
    let mut cursor = 0;
    for piece in pieces {
        let length = piece_len(piece);
        if let InlinePiece::Text { text, marks } = piece {
            if !text.is_empty() && offset >= cursor && offset < cursor + length {
                return marks.clone();
            }
            if !text.is_empty() && offset == cursor {
                return marks.clone();
            }
        }
        cursor += length;
    }
    pieces.iter().rev().find_map(|piece| match piece {
        InlinePiece::Text { marks, .. } => marks.clone(),
        InlinePiece::Atom(_) => None,
    })
}

fn insert_plain(
    pieces: &mut Vec<InlinePiece>,
    offset: usize,
    text: &str,
    marks: Option<Vec<ProseMirrorMark>>,
) {
    split_pieces_at(pieces, offset);
    let mut cursor = 0;
    let mut index = pieces.len();
    for (position, piece) in pieces.iter().enumerate() {
        if cursor == offset {
            index = position;
            break;
        }
        cursor += piece_len(piece);
    }
    let inherited = marks.or_else(|| {
        pieces[..index]
            .iter()
            .rev()
            .find_map(|piece| match piece {
                InlinePiece::Text { marks, .. } => Some(marks.clone()),
                InlinePiece::Atom(_) => None,
            })
            .flatten()
            .or_else(|| {
                pieces[index..]
                    .iter()
                    .find_map(|piece| match piece {
                        InlinePiece::Text { marks, .. } => Some(marks.clone()),
                        InlinePiece::Atom(_) => None,
                    })
                    .flatten()
            })
    });
    pieces.insert(
        index,
        InlinePiece::Text {
            text: text.to_owned(),
            marks: inherited,
        },
    );
}

fn delete_range(pieces: &mut Vec<InlinePiece>, start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    let mut cursor = 0;
    let mut kept = Vec::new();
    let mut changed = false;
    for piece in pieces.drain(..) {
        match piece {
            InlinePiece::Text { text, marks } => {
                let length = text.chars().count();
                let piece_start = cursor;
                let piece_end = cursor + length;
                if piece_end <= start || piece_start >= end {
                    kept.push(InlinePiece::Text { text, marks });
                } else {
                    changed = true;
                    let local_start = start.saturating_sub(piece_start).min(length);
                    let local_end = end.saturating_sub(piece_start).min(length);
                    let (before, rest) = split_text(&text, local_start);
                    let (_, after) = split_text(&rest, local_end.saturating_sub(local_start));
                    if !before.is_empty() {
                        kept.push(InlinePiece::Text {
                            text: before,
                            marks: marks.clone(),
                        });
                    }
                    if !after.is_empty() {
                        kept.push(InlinePiece::Text { text: after, marks });
                    }
                }
                cursor += length;
            }
            InlinePiece::Atom(node) => {
                if cursor >= start && cursor < end {
                    changed = true;
                } else {
                    kept.push(InlinePiece::Atom(node));
                }
                cursor += 1;
            }
        }
    }
    if changed {
        *pieces = kept;
    }
    changed
}

fn split_pieces(pieces: Vec<InlinePiece>, offset: usize) -> (Vec<InlinePiece>, Vec<InlinePiece>) {
    let mut pieces = pieces;
    split_pieces_at(&mut pieces, offset);
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut cursor = 0;
    for piece in pieces {
        if cursor < offset {
            cursor += piece_len(&piece);
            before.push(piece);
        } else {
            after.push(piece);
        }
    }
    (before, after)
}

fn apply_mark(
    pieces: &mut Vec<InlinePiece>,
    start: usize,
    end: usize,
    mark: &ProseMirrorMark,
) -> bool {
    split_pieces_at(pieces, start);
    split_pieces_at(pieces, end);
    let mut cursor = 0;
    let mut all_have = true;
    let mut any_text = false;
    for piece in pieces.iter() {
        let length = piece_len(piece);
        if let InlinePiece::Text { text, marks } = piece {
            if !text.is_empty() && cursor >= start && cursor + length <= end {
                any_text = true;
                if !has_mark(marks, mark) {
                    all_have = false;
                }
            }
        }
        cursor += length;
    }
    if !any_text {
        return false;
    }
    let add = !all_have;
    let mut cursor = 0;
    for piece in pieces.iter_mut() {
        let length = piece_len(piece);
        if let InlinePiece::Text { text, marks } = piece {
            if !text.is_empty() && cursor >= start && cursor + length <= end {
                if add {
                    add_mark(marks, mark.clone());
                } else {
                    remove_mark(marks, mark);
                }
            }
        }
        cursor += length;
    }
    true
}

fn has_mark(marks: &Option<Vec<ProseMirrorMark>>, mark: &ProseMirrorMark) -> bool {
    marks
        .as_ref()
        .is_some_and(|marks| marks.iter().any(|candidate| candidate == mark))
}

fn add_mark(marks: &mut Option<Vec<ProseMirrorMark>>, mark: ProseMirrorMark) {
    let list = marks.get_or_insert_with(Vec::new);
    if !list.iter().any(|candidate| candidate == &mark) {
        list.push(mark);
    }
    *marks = normalize_marks(marks.take());
}

fn remove_mark(marks: &mut Option<Vec<ProseMirrorMark>>, mark: &ProseMirrorMark) {
    if let Some(list) = marks {
        list.retain(|candidate| candidate != mark);
        if list.is_empty() {
            *marks = None;
        } else {
            *marks = normalize_marks(marks.take());
        }
    }
}

fn normalize_marks(marks: Option<Vec<ProseMirrorMark>>) -> Option<Vec<ProseMirrorMark>> {
    let mut marks = marks?;
    marks.sort_by(|left, right| {
        mark_rank(&left.mark_type)
            .cmp(&mark_rank(&right.mark_type))
            .then_with(|| mark_key(left).cmp(&mark_key(right)))
    });
    marks.dedup();
    (!marks.is_empty()).then_some(marks)
}

fn mark_rank(mark_type: &str) -> u8 {
    match mark_type {
        "bold" => 0,
        "italic" => 1,
        "strike" => 2,
        "code" => 3,
        "link" => 4,
        _ => 5,
    }
}

fn mark_key(mark: &ProseMirrorMark) -> String {
    serde_json::to_string(&mark.attrs).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL_FIXTURE: &str =
        include_str!("../../../../core/core-db/tests/fixtures/block-schema-v1.json");

    fn editor() -> Editor {
        Editor::from_json_str(CANONICAL_FIXTURE).expect("canonical fixture")
    }

    fn text_at(editor: &Editor, path: &[usize]) -> String {
        editor
            .doc()
            .node(path)
            .map(|node| node.inline_text())
            .unwrap_or_default()
    }

    #[test]
    fn inserts_text_and_moves_the_caret() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![1], 0));
        assert!(editor.insert_text("INSERTED "));
        assert!(text_at(&editor, &[1]).starts_with("INSERTED Plain text"));
        assert_eq!(editor.selection().head, "INSERTED ".chars().count());
    }

    #[test]
    fn inserts_inside_a_marked_run_and_keeps_the_mark() {
        let mut editor = editor();
        // "Plain text with **bold**, ..." — the bold run starts at char 17.
        let path = vec![1];
        let text = text_at(&editor, &path);
        let start = text.find("bold").expect("bold word") as usize;
        editor.set_selection(Selection::caret(path.clone(), start + 2));
        assert!(editor.insert_text("XY"));
        let node = editor.doc().node(&path).expect("paragraph");
        let bold = node
            .children()
            .iter()
            .find(|child| child.text.as_deref() == Some("boXYld"))
            .expect("split bold run");
        assert!(has_mark(&bold.marks, &ProseMirrorMark::new("bold")));
    }

    #[test]
    fn deletes_backward_and_merges_adjacent_paragraphs() {
        let mut editor = Editor::from_json_str(
            r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"one"}]},{"type":"paragraph","content":[{"type":"text","text":"two"}]}]}"#,
        )
        .expect("doc");
        editor.set_selection(Selection::caret(vec![1], 0));
        assert!(editor.delete_backward());
        assert_eq!(editor.doc().content.len(), 1);
        assert_eq!(text_at(&editor, &[0]), "onetwo");
        assert_eq!(editor.selection().head, 3);
    }

    #[test]
    fn deletes_forward_and_merges_the_next_paragraph() {
        let mut editor = Editor::from_json_str(
            r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"one"}]},{"type":"paragraph","content":[{"type":"text","text":"two"}]}]}"#,
        )
        .expect("doc");
        editor.set_selection(Selection::caret(vec![0], 3));
        assert!(editor.delete_forward());
        assert_eq!(editor.doc().content.len(), 1);
        assert_eq!(text_at(&editor, &[0]), "onetwo");
    }

    #[test]
    fn splits_a_paragraph_and_continues_a_heading_as_a_paragraph() {
        let mut editor = editor();
        let original_len = editor.doc().content.len();
        // Heading "Phase 0 editor slice": split after "Phase".
        editor.set_selection(Selection::caret(vec![0], 5));
        assert!(editor.split_block());
        assert_eq!(editor.doc().content.len(), original_len + 1);
        assert_eq!(text_at(&editor, &[0]), "Phase");
        let second = editor.doc().node(&[1]).expect("second block");
        assert_eq!(second.node_type, "paragraph");
        assert_eq!(second.inline_text(), " 0 editor slice");
    }

    #[test]
    fn toggles_tasks_and_marks_and_undoes_both() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![5, 0, 0], 0));
        assert!(editor.toggle_task());
        let task = editor.doc().node(&[5, 0]).expect("task item");
        assert_eq!(
            task.attrs
                .as_ref()
                .and_then(|attrs| attrs.get("checked"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(editor.undo());
        let task = editor.doc().node(&[5, 0]).expect("task item");
        assert_eq!(
            task.attrs
                .as_ref()
                .and_then(|attrs| attrs.get("checked"))
                .and_then(Value::as_bool),
            Some(false)
        );

        editor.set_selection(Selection::range(vec![1], 0, 5));
        assert!(editor.toggle_mark(ProseMirrorMark::new("bold")));
        let paragraph = editor.doc().node(&[1]).expect("paragraph");
        assert!(paragraph
            .children()
            .iter()
            .any(|child| child.text.as_deref() == Some("Plain")
                && has_mark(&child.marks, &ProseMirrorMark::new("bold"))));
        assert!(editor.undo());
        let paragraph = editor.doc().node(&[1]).expect("paragraph");
        assert!(!paragraph
            .children()
            .iter()
            .any(|child| child.text.as_deref() == Some("Plain")
                && has_mark(&child.marks, &ProseMirrorMark::new("bold"))));
    }

    #[test]
    fn edits_a_table_cell_without_touching_the_table() {
        let mut editor = editor();
        // table row 0 -> header cell 0 -> paragraph 0
        editor.set_selection(Selection::caret(vec![9, 0, 0, 0], 0));
        assert!(editor.insert_text("X"));
        assert_eq!(text_at(&editor, &[9, 0, 0, 0]), "XColumn A");
        assert_eq!(text_at(&editor, &[9, 1, 0, 0]), "1");
        let json = editor.to_json_string();
        ProseMirrorDoc::from_json_str(&json).expect("the edited document is still valid");
    }

    #[test]
    fn preserves_unknown_nodes_and_round_trips_after_edits() {
        let mut editor = Editor::from_json_str(
            r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"hello"}]},{"type":"futureBlock","attrs":{"payload":{"a":1}}}]}"#,
        )
        .expect("doc");
        editor.set_selection(Selection::caret(vec![0], 5));
        assert!(editor.insert_text("!"));
        let json = editor.to_json_string();
        let doc = ProseMirrorDoc::from_json_str(&json).expect("valid document");
        assert_eq!(doc.content.len(), 2);
        assert_eq!(doc.content[1].node_type, "futureBlock");
        assert_eq!(
            doc.content[1]
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("payload")),
            Some(&json!({"a": 1}))
        );
    }

    #[test]
    fn replace_range_keeps_marks_outside_the_changed_span() {
        let mut editor = editor();
        let path = vec![1];
        let old = text_at(&editor, &path);
        let marker = old.find("bold").expect("bold");
        let mut next = old.clone();
        next.replace_range(marker..marker + 4, "BOLD");
        assert!(editor.replace_range(path.clone(), marker, marker + 4, "BOLD"));
        assert!(text_at(&editor, &path).contains("BOLD"));
        let node = editor.doc().node(&path).expect("paragraph");
        let bold = node
            .children()
            .iter()
            .find(|child| child.text.as_deref() == Some("BOLD"))
            .expect("replacement keeps the run");
        assert!(has_mark(&bold.marks, &ProseMirrorMark::new("bold")));
    }

    #[test]
    fn undo_and_redo_restore_document_and_selection() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![0], 0));
        let original = editor.to_json_string();
        assert!(editor.insert_text("A"));
        assert_ne!(editor.to_json_string(), original);
        assert!(editor.undo());
        assert_eq!(editor.to_json_string(), original);
        assert_eq!(editor.selection().head, 0);
        assert!(editor.redo());
        assert!(text_at(&editor, &[0]).starts_with('A'));
    }
}
