//! ORBIT UI: design tokens, geometry, painting, animations, and views.
//! No Win32 calls and no I/O live in this crate.

pub mod anim;
pub mod geometry;
pub mod paint;
pub mod pill;
pub mod theme;

pub use egui_phosphor::{fill as icons_fill, regular as icons};
