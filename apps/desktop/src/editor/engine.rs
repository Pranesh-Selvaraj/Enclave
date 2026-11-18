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

    /// Move the caret to the next or previous cell in the nearest table.
    /// Navigation wraps within the table and does not modify the document.
    pub fn move_table_cell(&mut self, backwards: bool) -> bool {
        let path = self.selection.path.clone();
        let Some(cell_path) = table_cell_ancestor(&self.doc, &path) else {
            return false;
        };
        let Some(table_path) = nearest_ancestor(&self.doc, &cell_path, "table") else {
            return false;
        };
        let Some(table) = self.doc.node(&table_path) else {
            return false;
        };
        let mut cells = Vec::new();
        for (row_index, row) in table.children().iter().enumerate() {
            if row.node_type != "tableRow" {
                continue;
            }
            for (cell_index, cell) in row.children().iter().enumerate() {
                if matches!(cell.node_type.as_str(), "tableCell" | "tableHeader") {
                    let mut cell_path = table_path.clone();
                    cell_path.push(row_index);
                    cell_path.push(cell_index);
                    cells.push(cell_path);
                }
            }
        }
        if cells.is_empty() {
            return false;
        }
        let Some(current) = cells.iter().position(|candidate| candidate == &cell_path) else {
            return false;
        };
        let target = if backwards {
            (current + cells.len() - 1) % cells.len()
        } else {
            (current + 1) % cells.len()
        };
        let Some(cell) = self.doc.node(&cells[target]) else {
            return false;
        };
        let mut next_path = cells[target].clone();
        next_path.extend(first_text_block_path(cell.children()).unwrap_or_default());
        self.selection = Selection::caret(next_path, 0);
        true
    }

    /// Number of rows and the widest row in the table containing the caret.
    pub fn table_dimensions(&self) -> Option<(usize, usize)> {
        let (table_path, _, _, _, _) = table_location(&self.doc, &self.selection.path)?;
        let table = self.doc.node(&table_path)?;
        Some((
            table.children().len(),
            table
                .children()
                .iter()
                .map(|row| row.children().len())
                .max()
                .unwrap_or(0),
        ))
    }

    /// Insert a new table after the top-level block containing the caret.
    pub fn insert_table(&mut self, rows: usize, columns: usize) -> bool {
        if rows == 0 || columns == 0 {
            return false;
        }
        let Some(root) = self.selection.path.first().copied() else {
            return false;
        };
        let before = self.snapshot();
        let table = ProseMirrorNode {
            node_type: "table".to_owned(),
            attrs: None,
            content: Some(
                (0..rows)
                    .map(|row_index| {
                        let cell_type = if row_index == 0 {
                            "tableHeader"
                        } else {
                            "tableCell"
                        };
                        ProseMirrorNode {
                            node_type: "tableRow".to_owned(),
                            attrs: None,
                            content: Some(
                                (0..columns)
                                    .map(|_| table_cell(cell_type))
                                    .collect::<Vec<_>>(),
                            ),
                            marks: None,
                            text: None,
                        }
                    })
                    .collect::<Vec<_>>(),
            ),
            marks: None,
            text: None,
        };
        if !self.doc.insert_sibling(&[root], table) {
            return false;
        }
        self.selection = Selection::caret(vec![root + 1, 0, 0, 0], 0);
        self.finish_edit(before);
        true
    }

    /// Add a data row below the row containing the caret.
    pub fn add_table_row(&mut self) -> bool {
        let Some((table_path, row_path, _, row_index, cell_index)) =
            table_location(&self.doc, &self.selection.path)
        else {
            return false;
        };
        let Some(row) = self.doc.node(&row_path) else {
            return false;
        };
        let columns = row.children().len();
        if columns == 0 {
            return false;
        }
        let before = self.snapshot();
        let new_row = ProseMirrorNode {
            node_type: "tableRow".to_owned(),
            attrs: None,
            content: Some(
                (0..columns)
                    .map(|_| table_cell("tableCell"))
                    .collect::<Vec<_>>(),
            ),
            marks: None,
            text: None,
        };
        if !self.doc.insert_sibling(&row_path, new_row) {
            return false;
        }
        let mut next_path = table_path;
        next_path.push(row_index + 1);
        next_path.push(cell_index.min(columns - 1));
        next_path.push(0);
        self.selection = Selection::caret(next_path, 0);
        self.finish_edit(before);
        true
    }

    /// Remove the row containing the caret, when at least one row remains.
    pub fn delete_table_row(&mut self) -> bool {
        let Some((table_path, row_path, _, row_index, cell_index)) =
            table_location(&self.doc, &self.selection.path)
        else {
            return false;
        };
        let Some(table) = self.doc.node(&table_path) else {
            return false;
        };
        let rows = table.children().len();
        if rows <= 1 {
            return false;
        }
        let target_row = row_index.min(rows.saturating_sub(2));
        let target_cells = table
            .children()
            .get(target_row)
            .map(|row| row.children().len())
            .unwrap_or(0);
        if target_cells == 0 {
            return false;
        }
        let before = self.snapshot();
        if self.doc.remove_node(&row_path).is_none() {
            return false;
        }
        let mut next_path = table_path;
        next_path.push(target_row);
        next_path.push(cell_index.min(target_cells - 1));
        next_path.push(0);
        self.selection = Selection::caret(next_path, 0);
        self.finish_edit(before);
        true
    }

    /// Add an empty cell to every row, immediately to the right of the caret.
    pub fn add_table_column(&mut self) -> bool {
        let Some((table_path, _, _, row_index, cell_index)) =
            table_location(&self.doc, &self.selection.path)
        else {
            return false;
        };
        let Some(table) = self.doc.node(&table_path).cloned() else {
            return false;
        };
        if table
            .children()
            .iter()
            .any(|row| row.children().len() < cell_index + 1)
        {
            return false;
        }
        let before = self.snapshot();
        for (index, row) in table.children().iter().enumerate() {
            let cell_type = if row
                .children()
                .first()
                .is_some_and(|cell| cell.node_type == "tableHeader")
            {
                "tableHeader"
            } else {
                "tableCell"
            };
            let mut target = table_path.clone();
            target.push(index);
            target.push(cell_index + 1);
            if !self.doc.insert_node(&target, table_cell(cell_type)) {
                return false;
            }
        }
        let mut next_path = table_path;
        next_path.push(row_index);
        next_path.push(cell_index + 1);
        next_path.push(0);
        self.selection = Selection::caret(next_path, 0);
        self.finish_edit(before);
        true
    }

    /// Remove the caret's column from every row, when at least one remains.
    pub fn delete_table_column(&mut self) -> bool {
        let Some((table_path, _, _, row_index, cell_index)) =
            table_location(&self.doc, &self.selection.path)
        else {
            return false;
        };
        let Some(table) = self.doc.node(&table_path) else {
            return false;
        };
        let columns = table
            .children()
            .iter()
            .map(|row| row.children().len())
            .max()
            .unwrap_or(0);
        if columns <= 1
            || table
                .children()
                .iter()
                .any(|row| row.children().len() <= cell_index)
        {
            return false;
        }
        let before = self.snapshot();
        for index in 0..table.children().len() {
            let mut target = table_path.clone();
            target.push(index);
            target.push(cell_index);
            if self.doc.remove_node(&target).is_none() {
                return false;
            }
        }
        let mut next_path = table_path;
        next_path.push(row_index);
        next_path.push(cell_index.min(columns.saturating_sub(2)));
        next_path.push(0);
        self.selection = Selection::caret(next_path, 0);
        self.finish_edit(before);
        true
    }

    /// Move the current list item under its previous sibling as a nested list.
    pub fn indent_list_item(&mut self) -> bool {
        let path = self.selection.path.clone();
        let Some((item_path, list_path, item_index)) = single_child_list_item(&self.doc, &path)
        else {
            return false;
        };
        if item_index == 0 {
            return false;
        }
        let Some(item) = self.doc.node(&item_path).cloned() else {
            return false;
        };
        let Some(list) = self.doc.node(&list_path).cloned() else {
            return false;
        };
        let first_content = first_text_block_path(item.children()).unwrap_or_default();
        let mut previous_item_path = list_path.clone();
        previous_item_path.push(item_index - 1);
        if self.doc.node(&previous_item_path).is_none() {
            return false;
        }
        if self.doc.remove_node(&item_path).is_none() {
            return false;
        }
        let Some(previous_item) = self.doc.node_mut(&previous_item_path) else {
            return false;
        };
        let children = previous_item.content.get_or_insert_with(Vec::new);
        let (nested_index, new_index, has_existing) = match children.last() {
            Some(last) if last.node_type == list.node_type => {
                (children.len() - 1, last.children().len(), true)
            }
            _ => (children.len(), 0, false),
        };
        if has_existing {
            if let Some(nested) = children[nested_index].content.as_mut() {
                nested.push(item);
            } else {
                children[nested_index].content = Some(vec![item]);
            }
        } else {
            let mut nested = ProseMirrorNode {
                node_type: list.node_type,
                attrs: list.attrs,
                content: Some(vec![item]),
                marks: None,
                text: None,
            };
            // Keep the list type but do not copy ordering start attributes onto
            // a freshly nested list; the surrounding list owns those.
            if nested.node_type == "orderedList" {
                nested.attrs = None;
            }
            children.push(nested);
        }
        let mut next_path = previous_item_path;
        next_path.push(nested_index);
        next_path.push(new_index);
        next_path.extend(first_content);
        self.selection = Selection::caret(next_path, 0);
        true
    }

    /// Move the current nested list item out to the parent list, after the item
    /// that contained its list.
    pub fn outdent_list_item(&mut self) -> bool {
        let path = self.selection.path.clone();
        let Some((item_path, list_path, _)) = single_child_list_item(&self.doc, &path) else {
            return false;
        };
        if list_path.len() < 3 {
            return false;
        }
        let Some(item) = self.doc.node(&item_path).cloned() else {
            return false;
        };
        let first_content = first_text_block_path(item.children()).unwrap_or_default();
        let parent_item_path = list_path[..list_path.len() - 1].to_vec();
        let parent_list_path = parent_item_path[..parent_item_path.len() - 1].to_vec();
        let Some(parent_index) = parent_item_path.last().copied() else {
            return false;
        };
        if self.doc.remove_node(&item_path).is_none() {
            return false;
        }
        if self
            .doc
            .node(&list_path)
            .is_some_and(|nested| nested.children().is_empty())
        {
            self.doc.remove_node(&list_path);
        }
        let mut target = parent_list_path;
        target.push(parent_index + 1);
        if !self.doc.insert_node(&target, item) {
            return false;
        }
        target.extend(first_content);
        self.selection = Selection::caret(target, 0);
        true
    }

    /// Wrap the current text block in a list, or change the list type when the
    /// caret is already inside a list item. Returns false for an unchanged type
    /// or an unsupported block.
    pub fn set_list_type(&mut self, list_type: &str) -> bool {
        if !matches!(list_type, "bulletList" | "orderedList" | "taskList") {
            return false;
        }
        let path = self.selection.path.clone();
        let offset = self.selection.head;
        if let Some((item_path, list_path, _)) = single_child_list_item(&self.doc, &path) {
            let Some(list) = self.doc.node(&list_path) else {
                return false;
            };
            if list.node_type == list_type {
                return false;
            }
            let Some(list) = self.doc.node_mut(&list_path) else {
                return false;
            };
            list.node_type = list_type.to_owned();
            list.attrs = None;
            let Some(item) = self.doc.node_mut(&item_path) else {
                return false;
            };
            item.node_type = if list_type == "taskList" {
                "taskItem".to_owned()
            } else {
                "listItem".to_owned()
            };
            item.attrs = if list_type == "taskList" {
                Some(json!({ "checked": false }))
            } else {
                None
            };
            return true;
        }

        let Some(block) = self.doc.node(&path).cloned() else {
            return false;
        };
        if !block.is_text_block() {
            return false;
        }
        let item = ProseMirrorNode {
            node_type: if list_type == "taskList" {
                "taskItem".to_owned()
            } else {
                "listItem".to_owned()
            },
            attrs: (list_type == "taskList").then(|| json!({ "checked": false })),
            content: Some(vec![block]),
            marks: None,
            text: None,
        };
        let list = ProseMirrorNode {
            node_type: list_type.to_owned(),
            attrs: None,
            content: Some(vec![item]),
            marks: None,
            text: None,
        };
        let Some(slot) = self.doc.node_mut(&path) else {
            return false;
        };
        *slot = list;
        let mut next_path = path;
        next_path.push(0);
        next_path.push(0);
        self.selection = Selection::caret(next_path, offset);
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
        if self.remove_empty_list_item() {
            return true;
        }
        self.merge_previous()
    }

    /// Backspace in an empty list item removes the item. The first item of a
    /// single-item list exits the list and leaves a paragraph in its place.
    fn remove_empty_list_item(&mut self) -> bool {
        let path = self.selection.path.clone();
        let Some((item_path, list_path, item_index)) = single_child_list_item(&self.doc, &path)
        else {
            return false;
        };
        let Some(block) = self.doc.node(&path) else {
            return false;
        };
        if !block.inline_text().is_empty() {
            return false;
        }
        let Some(list) = self.doc.node(&list_path).cloned() else {
            return false;
        };
        if list.children().len() == 1 {
            // Remove the wrapper: the empty paragraph becomes the block at the
            // list's old index. Clone it before removing the list.
            let Some(paragraph) = self
                .doc
                .node(&item_path)
                .and_then(|item| item.children().first().cloned())
            else {
                return false;
            };
            if self.doc.remove_node(&list_path).is_none() {
                return false;
            }
            if !self.doc.insert_node(&list_path, paragraph) {
                return false;
            }
            self.selection = Selection::caret(list_path, 0);
            return true;
        }

        let previous_path = if item_index > 0 {
            let mut previous = list_path.clone();
            previous.push(item_index - 1);
            previous.push(0);
            Some(previous)
        } else {
            None
        };
        let previous_len = previous_path
            .as_deref()
            .and_then(|path| self.doc.node(path))
            .map(|node| node.inline_text().chars().count())
            .unwrap_or(0);
        if self.doc.remove_node(&item_path).is_none() {
            return false;
        }
        self.selection = match previous_path {
            Some(path) => Selection::caret(path, previous_len),
            None => {
                let mut next = list_path;
                next.push(0);
                next.push(0);
                Selection::caret(next, 0)
            }
        };
        true
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

        if let Some((item_path, list_path, item_index)) = single_child_list_item(&self.doc, &path) {
            let Some(item) = self.doc.node(&item_path).cloned() else {
                return false;
            };
            let mut first_item = item.clone();
            first_item.content = Some(vec![first]);
            let mut new_item = item;
            if new_item.node_type == "taskItem" {
                let attrs = new_item.attrs.get_or_insert_with(|| json!({}));
                if let Value::Object(map) = attrs {
                    map.insert("checked".to_owned(), Value::Bool(false));
                } else {
                    new_item.attrs = Some(json!({ "checked": false }));
                }
            }
            new_item.content = Some(vec![second]);
            if !self.doc.insert_sibling(&item_path, new_item) {
                return false;
            }
            let Some(item_node) = self.doc.node_mut(&item_path) else {
                return false;
            };
            *item_node = first_item;
            let mut next_path = list_path;
            next_path.push(item_index + 1);
            next_path.push(0);
            self.selection = Selection::caret(next_path, 0);
            return true;
        }

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

fn table_cell(node_type: &str) -> ProseMirrorNode {
    ProseMirrorNode {
        node_type: node_type.to_owned(),
        attrs: None,
        content: Some(vec![ProseMirrorNode::paragraph(Vec::new())]),
        marks: None,
        text: None,
    }
}

/// Table, row, cell and their indexes for the cell containing `path`.
fn table_location(
    doc: &ProseMirrorDoc,
    path: &[usize],
) -> Option<(NodePath, NodePath, NodePath, usize, usize)> {
    let cell_path = table_cell_ancestor(doc, path)?;
    let row_path = cell_path[..cell_path.len() - 1].to_vec();
    let row = doc.node(&row_path)?;
    if row.node_type != "tableRow" {
        return None;
    }
    let table_path = row_path[..row_path.len() - 1].to_vec();
    let table = doc.node(&table_path)?;
    if table.node_type != "table" {
        return None;
    }
    let row_index = *row_path.last()?;
    let cell_index = *cell_path.last()?;
    Some((table_path, row_path, cell_path, row_index, cell_index))
}

fn table_cell_ancestor(doc: &ProseMirrorDoc, path: &[usize]) -> Option<NodePath> {
    for depth in (0..path.len()).rev() {
        let prefix = &path[..=depth];
        let node = doc.node(prefix)?;
        if matches!(node.node_type.as_str(), "tableCell" | "tableHeader") {
            return Some(prefix.to_vec());
        }
    }
    None
}

fn nearest_ancestor(doc: &ProseMirrorDoc, path: &[usize], node_type: &str) -> Option<NodePath> {
    for depth in (0..path.len()).rev() {
        let prefix = &path[..=depth];
        if doc
            .node(prefix)
            .is_some_and(|node| node.node_type == node_type)
        {
            return Some(prefix.to_vec());
        }
    }
    None
}

/// If the caret sits in the only paragraph of a list/task item, return the
/// item, its parent list and the item's index. Splitting there creates a new
/// sibling item rather than a second paragraph inside the same item.
fn single_child_list_item(
    doc: &ProseMirrorDoc,
    path: &[usize],
) -> Option<(NodePath, NodePath, usize)> {
    for depth in (0..path.len()).rev() {
        let item_path = &path[..=depth];
        let item = doc.node(item_path)?;
        if !matches!(item.node_type.as_str(), "listItem" | "taskItem") {
            continue;
        }
        let item_children = item.children();
        if item_children.len() != 1
            || path.len() != item_path.len() + 1
            || path.get(item_path.len()) != Some(&0)
        {
            return None;
        }
        let list_path = item_path[..item_path.len() - 1].to_vec();
        let list = doc.node(&list_path)?;
        if !matches!(
            list.node_type.as_str(),
            "bulletList" | "orderedList" | "taskList"
        ) {
            return None;
        }
        return Some((item_path.to_vec(), list_path, *item_path.last()?));
    }
    None
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
    fn inserts_a_table_after_the_current_block() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![1], 3));
        assert!(editor.insert_table(2, 3));
        let table = editor.doc().node(&[2]).expect("table");
        assert_eq!(table.node_type, "table");
        assert_eq!(table.children().len(), 2);
        assert_eq!(table.children()[0].children().len(), 3);
        assert_eq!(table.children()[0].children()[0].node_type, "tableHeader");
        assert_eq!(table.children()[1].children()[0].node_type, "tableCell");
        assert_eq!(editor.selection().path, vec![2, 0, 0, 0]);
    }

    #[test]
    fn adds_and_removes_table_rows_and_columns() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![9, 0, 0, 0], 0));
        assert_eq!(editor.table_dimensions(), Some((2, 2)));
        assert!(editor.add_table_row());
        assert_eq!(editor.doc().node(&[9]).expect("table").children().len(), 3);
        assert_eq!(editor.selection().path, vec![9, 1, 0, 0]);
        assert!(editor.add_table_column());
        assert_eq!(editor.table_dimensions(), Some((3, 3)));
        assert_eq!(editor.selection().path, vec![9, 1, 1, 0]);
        assert!(editor.delete_table_column());
        assert_eq!(editor.table_dimensions(), Some((3, 2)));
        assert!(editor.delete_table_row());
        assert_eq!(editor.table_dimensions(), Some((2, 2)));
        assert!(editor.delete_table_row());
        assert!(!editor.delete_table_row());
        assert!(editor.delete_table_column());
        assert!(!editor.delete_table_column());
    }

    #[test]
    fn table_edits_are_undoable() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![9, 0, 0, 0], 0));
        assert!(editor.add_table_row());
        assert!(editor.can_undo());
        assert!(editor.undo());
        assert_eq!(editor.table_dimensions(), Some((2, 2)));
    }

    #[test]
    fn cannot_delete_the_last_table_row_or_column() {
        let mut editor = Editor::from_json_str(
            r#"{"type":"doc","content":[{"type":"table","content":[{"type":"tableRow","content":[{"type":"tableHeader","content":[{"type":"paragraph"}]}]}]}]}"#,
        )
        .expect("doc");
        editor.set_selection(Selection::caret(vec![0, 0, 0, 0], 0));
        assert!(!editor.delete_table_row());
        assert!(!editor.delete_table_column());
    }

    #[test]
    fn moves_between_table_cells_and_wraps() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![9, 0, 0, 0], 0));
        assert!(editor.move_table_cell(false));
        assert_eq!(editor.selection().path, vec![9, 0, 1, 0]);
        assert!(editor.move_table_cell(false));
        assert_eq!(editor.selection().path, vec![9, 1, 0, 0]);
        assert!(editor.move_table_cell(false));
        assert_eq!(editor.selection().path, vec![9, 1, 1, 0]);
        assert!(editor.move_table_cell(false));
        assert_eq!(editor.selection().path, vec![9, 0, 0, 0]);
        assert!(editor.move_table_cell(true));
        assert_eq!(editor.selection().path, vec![9, 1, 1, 0]);
    }

    #[test]
    fn table_navigation_does_not_change_the_history() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![9, 0, 0, 0], 0));
        assert!(editor.move_table_cell(false));
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn splitting_a_list_item_creates_a_new_item() {
        let mut editor = editor();
        // First bullet: "First bullet" -> "First" / " bullet".
        editor.set_selection(Selection::caret(vec![3, 0, 0], 5));
        assert!(editor.split_block());
        let list = editor.doc().node(&[3]).expect("bullet list");
        assert_eq!(list.children().len(), 3);
        assert_eq!(text_at(&editor, &[3, 0, 0]), "First");
        assert_eq!(text_at(&editor, &[3, 1, 0]), " bullet");
        assert_eq!(text_at(&editor, &[3, 2, 0]), "Second bullet");
        assert_eq!(editor.selection().path, vec![3, 1, 0]);
    }

    #[test]
    fn backspace_removes_an_empty_list_item() {
        let mut editor = editor();
        // Split at the end so the new item starts empty, then Backspace there.
        editor.set_selection(Selection::caret(
            vec![3, 0, 0],
            "First bullet".chars().count(),
        ));
        assert!(editor.split_block());
        editor.set_selection(Selection::caret(vec![3, 1, 0], 0));
        assert!(editor.delete_backward());
        let list = editor.doc().node(&[3]).expect("bullet list");
        assert_eq!(list.children().len(), 2);
        assert_eq!(editor.selection().path, vec![3, 0, 0]);
        assert_eq!(editor.selection().head, "First bullet".chars().count());
    }

    #[test]
    fn backspace_removes_the_first_empty_item_and_keeps_the_rest() {
        let mut editor = Editor::from_json_str(
            r#"{"type":"doc","content":[{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph"}]},{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Two"}]}]}]}]}"#,
        )
        .expect("doc");
        editor.set_selection(Selection::caret(vec![0, 0, 0], 0));
        assert!(editor.delete_backward());
        assert_eq!(editor.doc().node(&[0]).expect("list").children().len(), 1);
        assert_eq!(text_at(&editor, &[0, 0, 0]), "Two");
        assert_eq!(editor.selection().path, vec![0, 0, 0]);
    }

    #[test]
    fn backspace_exits_a_single_item_list() {
        let mut editor = Editor::from_json_str(
            r#"{"type":"doc","content":[{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph"}]}]}]}"#,
        )
        .expect("doc");
        editor.set_selection(Selection::caret(vec![0, 0, 0], 0));
        assert!(editor.delete_backward());
        assert_eq!(editor.doc().content.len(), 1);
        assert_eq!(
            editor.doc().node(&[0]).expect("paragraph").node_type,
            "paragraph"
        );
        assert_eq!(editor.selection().path, vec![0]);
    }

    #[test]
    fn wraps_a_paragraph_in_a_list_and_keeps_the_caret() {
        let mut editor = editor();
        let original = text_at(&editor, &[1]);
        editor.set_selection(Selection::caret(vec![1], 5));
        assert!(editor.set_list_type("bulletList"));
        assert_eq!(
            editor.doc().node(&[1]).expect("list").node_type,
            "bulletList"
        );
        assert_eq!(text_at(&editor, &[1, 0, 0]), original);
        assert_eq!(editor.selection().path, vec![1, 0, 0]);
        assert_eq!(editor.selection().head, 5);
        assert!(!editor.set_list_type("bulletList"));
    }

    #[test]
    fn changes_list_type_and_task_items_stay_unchecked() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![3, 0, 0], 0));
        assert!(editor.set_list_type("taskList"));
        assert_eq!(
            editor.doc().node(&[3]).expect("task list").node_type,
            "taskList"
        );
        let item = editor.doc().node(&[3, 0]).expect("task item");
        assert_eq!(item.node_type, "taskItem");
        assert_eq!(
            item.attrs
                .as_ref()
                .and_then(|attrs| attrs.get("checked"))
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(editor.set_list_type("bulletList"));
        assert_eq!(
            editor.doc().node(&[3, 0]).expect("item").node_type,
            "listItem"
        );
        assert!(editor
            .doc()
            .node(&[3, 0])
            .expect("item")
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("checked"))
            .is_none());
    }

    #[test]
    fn indents_and_outdents_a_list_item() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![3, 1, 0], 0));
        assert!(editor.indent_list_item());
        let root = editor.doc().node(&[3]).expect("parent list");
        assert_eq!(root.children().len(), 1);
        assert_eq!(text_at(&editor, &[3, 0, 1, 0, 0]), "Second bullet");
        assert_eq!(editor.selection().path, vec![3, 0, 1, 0, 0]);
        assert!(editor.outdent_list_item());
        assert_eq!(
            editor.doc().node(&[3]).expect("root list").children().len(),
            2
        );
        assert!(editor.doc().node(&[3, 0, 1]).is_none());
        assert_eq!(text_at(&editor, &[3, 1, 0]), "Second bullet");
        assert_eq!(editor.selection().path, vec![3, 1, 0]);
    }

    #[test]
    fn the_first_list_item_cannot_indent() {
        let mut editor = editor();
        editor.set_selection(Selection::caret(vec![3, 0, 0], 0));
        assert!(!editor.indent_list_item());
        assert_eq!(editor.doc().node(&[3]).expect("list").children().len(), 2);
    }

    #[test]
    fn splitting_a_task_item_creates_an_unchecked_task() {
        let mut editor = editor();
        // First task: "Todo item" -> "Todo" / " item".
        editor.set_selection(Selection::caret(vec![5, 0, 0], 4));
        assert!(editor.split_block());
        let list = editor.doc().node(&[5]).expect("task list");
        assert_eq!(list.children().len(), 3);
        assert_eq!(text_at(&editor, &[5, 0, 0]), "Todo");
        assert_eq!(text_at(&editor, &[5, 1, 0]), " item");
        let new_task = editor.doc().node(&[5, 1]).expect("new task item");
        assert_eq!(
            new_task
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("checked"))
                .and_then(Value::as_bool),
            Some(false)
        );
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
