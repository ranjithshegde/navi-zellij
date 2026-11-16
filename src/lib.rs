//! Native Vim-style navigation across Zellij and Neovim.
//!
//! The library holds the shell key parser and the interception eligibility check.
//! The Zellij plugin entry point lives in `main.rs`.

mod keys;
mod pairs;
mod state;

pub use keys::{Effect, KeyRouter, NavAction, ResizeAction};
pub use pairs::{is_pairable_terminal, parse_request, PairRegistry, PairRequest};
pub use state::should_intercept;
pub use zellij_tile::prelude::Direction;
