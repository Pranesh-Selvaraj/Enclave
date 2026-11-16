//! The headless editor engine.
//!
//! `model` owns the ProseMirror contract and validation; `selection` and
//! `engine` own editing state and commands. Nothing here depends on egui, so
//! the view and the future AI tools share one implementation.

#[allow(dead_code)] // the view and AI tools consume the editing commands as they land
mod engine;
#[allow(dead_code)] // the model is cohesive; the view consumes more of it next
mod model;
#[allow(dead_code)] // the view uses carets now; ranges land with mark selection
mod selection;

pub use engine::Editor;
pub use model::{NodePath, ProseMirrorDoc, ProseMirrorMark, ProseMirrorNode};
pub use selection::Selection;
