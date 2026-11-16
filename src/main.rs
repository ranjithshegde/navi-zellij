//! Headless Zellij plugin that routes shell shortcuts through the key parser.

use navi_zellij::{should_intercept, Effect, KeyRouter, NavAction, ResizeAction};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use zellij_tile::prelude::*;

/// Seconds between client and foreground-command refreshes.
const POLL: f64 = 0.4;
/// Seconds to wait for the second key of a shell shortcut.
const PREFIX: f64 = 0.18;

/// Plugin state for the focused pane, permission grant, and buffered prefix.
#[derive(Default)]
struct NaviZellij {
    mode: Option<ModeInfo>,
    focused_pane: Option<PaneId>,
    intercepting: bool,
    permission_granted: bool,
    router: KeyRouter,
    pending_since: Option<Instant>,
}

impl NaviZellij {
    /// Release interception and discard a prefix when the destination is uncertain.
    fn stop(&mut self) {
        if self.intercepting {
            clear_key_presses_intercepts();
        }
        self.intercepting = false;
        self.focused_pane = None;
        self.router = KeyRouter::default();
        self.pending_since = None;
    }

    /// Enable interception only for one known shell client in Locked mode.
    fn update_clients(&mut self, clients: Vec<ClientInfo>) {
        let ready = self.permission_granted
            && self
                .mode
                .as_ref()
                .is_some_and(|m| m.session_name.is_some() && should_intercept(m.mode, &clients));
        if !ready {
            self.stop();
            return;
        }
        let pane = clients[0].pane_id;
        if self.focused_pane != Some(pane) {
            self.stop();
            self.focused_pane = Some(pane);
        }
        if !self.intercepting {
            intercept_key_presses();
            self.intercepting = true;
        }
    }

    /// Find the user's existing Locked-mode binding for an intercepted key.
    fn focused_binding(&self, key: &KeyWithModifier) -> Option<Vec<actions::Action>> {
        self.mode
            .as_ref()?
            .keybinds
            .iter()
            .find(|(mode, _)| *mode == InputMode::Locked)?
            .1
            .iter()
            .find(|(bound, _)| bound == key)
            .map(|(_, actions)| actions.clone())
    }

    /// Forward ordinary input or run the parsed Zellij action.
    fn dispatch(&mut self, effect: Effect) {
        match effect {
            Effect::Forward(key) => {
                if let (Some(mode), Some(pane)) = (&self.mode, self.focused_pane) {
                    if let Some(session) = &mode.session_name {
                        send_key_to_session_pane(session, pane, key);
                    }
                }
            }
            Effect::Action(action) => match action {
                NavAction::Focus(direction) => move_focus(direction),
                NavAction::Split(direction) => run_action(
                    actions::Action::NewPane {
                        direction: Some(direction),
                        pane_name: None,
                        start_suppressed: false,
                    },
                    BTreeMap::new(),
                ),
                NavAction::Close => close_focus(),
                NavAction::NextPane => focus_next_pane(),
                NavAction::PreviousPane => focus_previous_pane(),
                NavAction::LastPane => focus_last_pane(),
                NavAction::Resize(ResizeAction::Grow) => resize_focused_pane(Resize::Increase),
                NavAction::Resize(ResizeAction::Shrink) => resize_focused_pane(Resize::Decrease),
                NavAction::Resize(ResizeAction::GrowRight) => {
                    resize_focused_pane_with_direction(Resize::Increase, Direction::Right)
                }
                NavAction::Resize(ResizeAction::ShrinkRight) => {
                    resize_focused_pane_with_direction(Resize::Decrease, Direction::Right)
                }
                NavAction::NextTab => go_to_next_tab(),
                NavAction::PreviousTab => go_to_previous_tab(),
            },
        }
    }

    /// Give configured Zellij bindings priority, then parse shell shortcuts.
    fn on_key(&mut self, key: KeyWithModifier) {
        if !self.intercepting || self.focused_pane.is_none() {
            return;
        }
        if !self.router.is_quoting() {
            if let Some(binding) = self.focused_binding(&key) {
                for effect in self.router.timeout() {
                    self.dispatch(effect);
                }
                self.pending_since = None;
                // A mode switch should release interception before Zellij executes it.
                if binding
                    .iter()
                    .any(|a| matches!(a, actions::Action::SwitchToMode { .. }))
                {
                    self.stop();
                }
                for action in binding {
                    run_action(action, BTreeMap::new());
                }
                list_clients();
                return;
            }
        }
        for effect in self.router.handle(key) {
            self.dispatch(effect);
        }
        if self.router.is_pending() {
            self.pending_since = Some(Instant::now());
            set_timeout(PREFIX);
        } else {
            self.pending_since = None;
        }
    }

    /// Flush the current prefix only after its full delay has elapsed.
    fn on_prefix_timeout(&mut self) {
        let Some(started) = self.pending_since else {
            return;
        };
        let elapsed = started.elapsed();
        if elapsed < Duration::from_secs_f64(PREFIX) {
            set_timeout((Duration::from_secs_f64(PREFIX) - elapsed).as_secs_f64());
            return;
        }
        for effect in self.router.timeout() {
            self.dispatch(effect);
        }
        self.pending_since = None;
    }
}

impl ZellijPlugin for NaviZellij {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        subscribe(&[
            EventType::ModeUpdate,
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::ListClients,
            EventType::PermissionRequestResult,
            EventType::InterceptedKeyPress,
            EventType::CommandChanged,
            EventType::Timer,
        ]);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::WriteToStdin,
            PermissionType::InterceptInput,
            PermissionType::RunActionsAsUser,
        ]);
        set_timeout(POLL);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::ModeUpdate(mode) => {
                if mode.mode != InputMode::Locked {
                    self.stop();
                }
                self.mode = Some(mode);
                list_clients();
            }
            Event::PaneUpdate(_) | Event::TabUpdate(_) => list_clients(),
            Event::ListClients(clients) => self.update_clients(clients),
            Event::PermissionRequestResult(status) => {
                self.permission_granted = status == PermissionStatus::Granted;
                if self.permission_granted {
                    list_clients();
                } else {
                    self.stop();
                }
            }
            Event::InterceptedKeyPress(key) => self.on_key(key),
            Event::CommandChanged(pane, _, _, _) => {
                if self.focused_pane == Some(pane) {
                    self.stop();
                }
                list_clients();
            }
            Event::Timer(value) if (value - POLL).abs() < 0.001 => {
                list_clients();
                set_timeout(POLL);
            }
            Event::Timer(_) => self.on_prefix_timeout(),
            _ => {}
        }
        false
    }
}

register_plugin!(NaviZellij);
