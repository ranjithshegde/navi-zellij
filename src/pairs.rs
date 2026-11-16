//! Session-local, one-to-one pairing of Zellij terminal panes.

use std::collections::{HashMap, HashSet};
use zellij_tile::prelude::PaneInfo;

/// Whether a Zellij pane can participate in a usable editor/shell pair.
pub fn is_pairable_terminal(pane: &PaneInfo) -> bool {
    !pane.is_plugin && !pane.exited && !pane.is_held && !pane.is_suppressed && pane.is_selectable
}

/// A request sent by the Neovim companion over the `navi-zellij-pair` pipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairRequest {
    /// Associate an editor terminal with a partner terminal.
    Pair(u32, u32),
    /// Return the terminal paired with this editor.
    Get(u32),
    /// Remove the pair containing this editor.
    Unpair(u32),
}

/// Parse a versioned pair request. Terminal IDs are nonnegative decimal integers.
pub fn parse_request(payload: &str) -> Result<PairRequest, &'static str> {
    let parts: Vec<_> = payload.split_whitespace().collect();
    if parts.first() != Some(&"v1") {
        return Err("unsupported pair protocol");
    }
    let id = |value: &str| value.parse::<u32>().map_err(|_| "invalid terminal ID");
    match parts.as_slice() {
        [_, "pair", editor, partner] => {
            let editor = id(editor)?;
            let partner = id(partner)?;
            if editor == partner {
                return Err("cannot pair a pane with itself");
            }
            Ok(PairRequest::Pair(editor, partner))
        }
        [_, "get", editor] => Ok(PairRequest::Get(id(editor)?)),
        [_, "unpair", editor] => Ok(PairRequest::Unpair(id(editor)?)),
        _ => Err("invalid pair request"),
    }
}

/// Bidirectional terminal-pane pairs. Pairing a used pane replaces its old pair.
#[derive(Debug, Default)]
pub struct PairRegistry {
    partners: HashMap<u32, u32>,
}

impl PairRegistry {
    /// Apply one CLI pipe request against the currently usable terminal IDs.
    /// The response is the complete line sent back to the caller.
    pub fn apply(
        &mut self,
        request: PairRequest,
        live: &HashSet<u32>,
    ) -> Result<String, &'static str> {
        match request {
            PairRequest::Pair(editor, partner) => {
                if !live.contains(&editor) || !live.contains(&partner) {
                    return Err("pane is no longer available");
                }
                self.pair(editor, partner)?;
                Ok("ok\n".into())
            }
            PairRequest::Get(editor) => {
                if let Some(partner) = self.get(editor) {
                    if live.contains(&editor) && live.contains(&partner) {
                        Ok(format!("pane {partner}\n"))
                    } else {
                        self.unpair(editor);
                        Ok("none\n".into())
                    }
                } else {
                    Ok("none\n".into())
                }
            }
            PairRequest::Unpair(editor) => {
                self.unpair(editor);
                Ok("ok\n".into())
            }
        }
    }

    /// Insert a pair, removing any pairs that contained either pane.
    pub fn pair(&mut self, editor: u32, partner: u32) -> Result<(), &'static str> {
        if editor == partner {
            return Err("invalid pair");
        }
        self.unpair(editor);
        self.unpair(partner);
        self.partners.insert(editor, partner);
        self.partners.insert(partner, editor);
        Ok(())
    }

    /// Return the pane paired with `pane`, if any.
    pub fn get(&self, pane: u32) -> Option<u32> {
        self.partners.get(&pane).copied()
    }

    /// Return a usable partner, removing a pair whose destination has become unavailable.
    pub fn available_partner(
        &mut self,
        pane: u32,
        is_available: impl FnOnce(u32) -> bool,
    ) -> Option<u32> {
        let partner = self.get(pane)?;
        if is_available(partner) {
            Some(partner)
        } else {
            self.unpair(pane);
            None
        }
    }

    /// Remove both directions of a pair. Missing pairs are harmless.
    pub fn unpair(&mut self, pane: u32) {
        if let Some(other) = self.partners.remove(&pane) {
            self.partners.remove(&other);
        }
    }

    /// Expire pairs when either pane is unavailable in the latest pane manifest.
    pub fn retain_live(&mut self, live: &HashSet<u32>) {
        let stale: Vec<_> = self
            .partners
            .iter()
            .filter_map(|(&pane, &other)| {
                (!live.contains(&pane) || !live.contains(&other)).then_some(pane)
            })
            .collect();
        for pane in stale {
            self.unpair(pane);
        }
    }
}
