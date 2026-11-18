//! Parse terminal key sequences into Zellij actions or keys to forward.
//!
//! In both profiles, an incomplete `Ctrl-w` returns to the program after a
//! short timer. Shells also buffer `g` for `gt` and `gT`.

use crate::state::InputProfile;
use zellij_tile::prelude::{BareKey, Direction, KeyModifier, KeyWithModifier};

/// Resize requests that correspond to supported Vim window commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResizeAction {
    /// Increase the focused pane size.
    Grow,
    /// Decrease the focused pane size.
    Shrink,
    /// Increase space at the right border.
    GrowRight,
    /// Decrease space at the right border.
    ShrinkRight,
}

/// A complete navigation command for the Zellij adapter to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavAction {
    /// Focus the pane in a direction.
    Focus(Direction),
    /// Open a pane in a direction.
    Split(Direction),
    /// Close the focused pane.
    Close,
    /// Focus the next pane.
    NextPane,
    /// Focus the previous pane.
    PreviousPane,
    /// Return to the last focused pane.
    LastPane,
    /// Resize the focused pane.
    Resize(ResizeAction),
    /// Switch to the next tab.
    NextTab,
    /// Switch to the previous tab.
    PreviousTab,
}

/// The parser result: forward a key unchanged or perform a Zellij action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Send the original key to the focused terminal pane.
    Forward(KeyWithModifier),
    /// Ask Zellij to perform a parsed navigation command.
    Action(NavAction),
}

/// Stateful parser for terminal `Ctrl-w` commands and shell-only tab shortcuts.
pub struct KeyRouter {
    pending: Option<Pending>,
    profile: InputProfile,
}

impl Default for KeyRouter {
    fn default() -> Self {
        Self::for_profile(InputProfile::Shell)
    }
}

enum Pending {
    CtrlW(KeyWithModifier),
    G(KeyWithModifier),
}

impl KeyRouter {
    /// Start with the shortcuts appropriate for a shell or another app.
    pub fn for_profile(profile: InputProfile) -> Self {
        Self {
            pending: None,
            profile,
        }
    }

    /// Whether a shortcut prefix needs a timeout before forwarding.
    pub fn is_pending(&self) -> bool {
        matches!(self.pending, Some(Pending::CtrlW(_) | Pending::G(_)))
    }

    /// Consume one intercepted key and return effects in delivery order.
    ///
    /// An empty result means the key is buffered as a possible prefix.
    pub fn handle(&mut self, key: KeyWithModifier) -> Vec<Effect> {
        match self.pending.take() {
            Some(Pending::CtrlW(prefix)) => match window_action(&key) {
                Some(action) => vec![Effect::Action(action)],
                None => vec![Effect::Forward(prefix), Effect::Forward(key)],
            },
            Some(Pending::G(prefix)) => match tab_action(&key) {
                Some(action) => vec![Effect::Action(action)],
                None => vec![Effect::Forward(prefix), Effect::Forward(key)],
            },
            None if is_ctrl(&key, 'w') => {
                self.pending = Some(Pending::CtrlW(key));
                Vec::new()
            }
            None if self.profile == InputProfile::Shell && plain_char(&key) == Some('g') => {
                self.pending = Some(Pending::G(key));
                Vec::new()
            }
            None => vec![Effect::Forward(key)],
        }
    }

    /// Release a buffered prefix when its shortcut window expires.
    pub fn timeout(&mut self) -> Vec<Effect> {
        match self.pending.take() {
            Some(Pending::CtrlW(key) | Pending::G(key)) => vec![Effect::Forward(key)],
            None => Vec::new(),
        }
    }
}

fn is_ctrl(key: &KeyWithModifier, c: char) -> bool {
    key.bare_key == BareKey::Char(c)
        && key.key_modifiers.len() == 1
        && key.key_modifiers.contains(&KeyModifier::Ctrl)
}

fn plain_char(key: &KeyWithModifier) -> Option<char> {
    if key.key_modifiers.is_empty() {
        if let BareKey::Char(c) = key.bare_key {
            return Some(c);
        }
    }
    None
}

fn shifted_char(key: &KeyWithModifier, c: char) -> bool {
    key.bare_key == BareKey::Char(c)
        && key.key_modifiers.len() == 1
        && key.key_modifiers.contains(&KeyModifier::Shift)
}

fn window_action(key: &KeyWithModifier) -> Option<NavAction> {
    let plain = plain_char(key);
    match plain {
        Some('h') => Some(NavAction::Focus(Direction::Left)),
        Some('j') => Some(NavAction::Focus(Direction::Down)),
        Some('k') => Some(NavAction::Focus(Direction::Up)),
        Some('l') => Some(NavAction::Focus(Direction::Right)),
        Some('s') => Some(NavAction::Split(Direction::Down)),
        Some('v') => Some(NavAction::Split(Direction::Right)),
        Some('c') => Some(NavAction::Close),
        Some('w') => Some(NavAction::NextPane),
        Some('p') => Some(NavAction::LastPane),
        Some('+') => Some(NavAction::Resize(ResizeAction::Grow)),
        Some('-') => Some(NavAction::Resize(ResizeAction::Shrink)),
        Some('>') => Some(NavAction::Resize(ResizeAction::GrowRight)),
        Some('<') => Some(NavAction::Resize(ResizeAction::ShrinkRight)),
        _ if shifted_char(key, 'w') || plain == Some('W') => Some(NavAction::PreviousPane),
        _ if shifted_char(key, '=') => Some(NavAction::Resize(ResizeAction::Grow)),
        _ if shifted_char(key, '.') => Some(NavAction::Resize(ResizeAction::GrowRight)),
        _ if shifted_char(key, ',') => Some(NavAction::Resize(ResizeAction::ShrinkRight)),
        _ => None,
    }
}

fn tab_action(key: &KeyWithModifier) -> Option<NavAction> {
    if plain_char(key) == Some('t') {
        Some(NavAction::NextTab)
    } else if shifted_char(key, 't') || plain_char(key) == Some('T') {
        Some(NavAction::PreviousTab)
    } else {
        None
    }
}
