//! Native Vim-style navigation across Zellij and Neovim.
//!
//! The library holds terminal key routing, foreground profiles, and pane state.
//! The Zellij plugin entry point lives in `main.rs`.

mod editor;
mod keys;
mod pairs;
mod state;

pub use editor::{parse_editor_signal, EditorRegistry, EditorSignal};
pub use keys::{Effect, KeyRouter, NavAction, ResizeAction};
pub use pairs::{is_pairable_terminal, parse_request, PairRegistry, PairRequest};
pub use state::{needs_command_lookup, profile_for_client, InputProfile};
pub use zellij_tile::prelude::Direction;
