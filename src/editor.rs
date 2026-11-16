//! Track Neovim's claim on a Zellij pane independently of command reports.

use std::collections::{HashMap, HashSet};

/// A lifecycle message sent by the Neovim companion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSignal {
    Enter {
        pane_id: u32,
        instance: String,
        sequence: u64,
    },
    Leave {
        pane_id: u32,
        instance: String,
        sequence: u64,
    },
}

/// Parse a versioned editor lifecycle message from a CLI pipe.
pub fn parse_editor_signal(payload: &str) -> Result<EditorSignal, &'static str> {
    let parts: Vec<_> = payload.split_whitespace().collect();
    if parts.first() != Some(&"v1") {
        return Err("unsupported editor protocol");
    }
    let [_, action, pane, instance, sequence] = parts.as_slice() else {
        return Err("invalid editor signal");
    };
    let pane_id = pane.parse().map_err(|_| "invalid terminal ID")?;
    let (started, pid) = instance.split_once(':').ok_or("invalid editor instance")?;
    if started.len() != 20
        || pid.len() != 10
        || !started.bytes().all(|digit| digit.is_ascii_digit())
        || !pid.bytes().all(|digit| digit.is_ascii_digit())
        || started.parse::<u64>().is_err()
        || pid.parse::<u32>().is_err()
    {
        return Err("invalid editor instance");
    }
    let sequence = sequence.parse().map_err(|_| "invalid editor sequence")?;
    if sequence == 0 {
        return Err("invalid editor sequence");
    }
    match *action {
        "enter" => Ok(EditorSignal::Enter {
            pane_id,
            instance: (*instance).into(),
            sequence,
        }),
        "leave" => Ok(EditorSignal::Leave {
            pane_id,
            instance: (*instance).into(),
            sequence,
        }),
        _ => Err("invalid editor signal"),
    }
}

struct EditorState {
    instance: String,
    sequence: u64,
    active: bool,
}

/// Session-local editor claims. Ordered instance IDs reject delayed older messages.
#[derive(Default)]
pub struct EditorRegistry {
    panes: HashMap<u32, EditorState>,
}

impl EditorRegistry {
    /// Apply one ordered lifecycle message.
    pub fn apply(&mut self, signal: EditorSignal) {
        let (pane_id, instance, sequence, active) = match signal {
            EditorSignal::Enter {
                pane_id,
                instance,
                sequence,
            } => (pane_id, instance, sequence, true),
            EditorSignal::Leave {
                pane_id,
                instance,
                sequence,
            } => (pane_id, instance, sequence, false),
        };
        let Some(state) = self.panes.get_mut(&pane_id) else {
            self.panes.insert(
                pane_id,
                EditorState {
                    instance,
                    sequence,
                    active,
                },
            );
            return;
        };
        if state.instance == instance {
            if sequence > state.sequence {
                state.sequence = sequence;
                state.active = active;
            }
        } else if instance > state.instance {
            state.instance = instance;
            state.sequence = sequence;
            state.active = active;
        }
    }

    /// Whether Neovim currently owns this terminal pane's input.
    pub fn contains(&self, pane_id: u32) -> bool {
        self.panes.get(&pane_id).is_some_and(|state| state.active)
    }

    /// Forget claims when their pane closes.
    pub fn retain_live(&mut self, live: &HashSet<u32>) {
        self.panes.retain(|pane_id, _| live.contains(pane_id));
    }
}
