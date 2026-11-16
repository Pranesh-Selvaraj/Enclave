//! A single-block selection.
//!
//! Cross-block selections and table-cell ranges are deliberately out of scope
//! for this slice. Offsets are Unicode scalar (char) indices, not bytes, so a
//! caller can never split a UTF-8 sequence by accident.

use crate::editor::model::NodePath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub path: NodePath,
    /// The non-moving end of the selection.
    pub anchor: usize,
    /// The moving end (the caret for typing).
    pub head: usize,
}

impl Selection {
    pub fn caret(path: NodePath, offset: usize) -> Self {
        Self {
            path,
            anchor: offset,
            head: offset,
        }
    }

    pub fn range(path: NodePath, anchor: usize, head: usize) -> Self {
        Self { path, anchor, head }
    }

    pub fn is_caret(&self) -> bool {
        self.anchor == self.head
    }

    /// The normalised range, regardless of selection direction.
    pub fn range_offsets(&self) -> (usize, usize) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn clamp(&mut self, text_len: usize) {
        self.anchor = self.anchor.min(text_len);
        self.head = self.head.min(text_len);
    }
}
