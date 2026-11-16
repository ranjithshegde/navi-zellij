//! Headless Zellij plugin that routes terminal shortcuts through the key parser.

use navi_zellij::{
    is_pairable_terminal, needs_command_lookup, parse_editor_signal, parse_request,
    profile_for_client, EditorRegistry, EditorSignal, Effect, InputProfile, KeyRouter, NavAction,
    PairRegistry, PairRequest, ResizeAction,
};
use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};
use zellij_tile::prelude::*;

/// Seconds between client and foreground-command refreshes.
const POLL: f64 = 0.4;
/// Seconds to wait for the second key of a shortcut.
const PREFIX: f64 = 0.18;

/// Plugin state for the focused pane, permission grant, and buffered prefix.
#[derive(Default)]
struct NaviZellij {
    mode: Option<ModeInfo>,
    focused_pane: Option<PaneId>,
    profile: Option<InputProfile>,
    intercepting: bool,
    permission_granted: bool,
    router: KeyRouter,
    pending_since: Option<Instant>,
    pairs: PairRegistry,
    editors: EditorRegistry,
}

impl NaviZellij {
    /// Release interception and discard a prefix when the destination is uncertain.
    fn stop(&mut self) {
        if self.intercepting {
            clear_key_presses_intercepts();
        }
        self.intercepting = false;
        self.focused_pane = None;
        self.profile = None;
        self.router = KeyRouter::default();
        self.pending_since = None;
    }

    /// Choose shell or app shortcuts for one known terminal client in Locked mode.
    fn update_clients(&mut self, clients: Vec<ClientInfo>) {
        // A shell or N/A label may hide the actual foreground application.
        let resolved_command = clients
            .first()
            .filter(|client| {
                self.permission_granted
                    && clients.len() == 1
                    && self
                        .mode
                        .as_ref()
                        .is_some_and(|m| m.mode == InputMode::Locked)
                    && needs_command_lookup(&client.running_command)
            })
            .and_then(|client| get_pane_running_command(client.pane_id).ok());
        let profile = self
            .mode
            .as_ref()
            .filter(|mode| self.permission_granted && mode.session_name.is_some())
            .and_then(|mode| {
                let editor_active = clients.first().is_some_and(|client| {
                    matches!(client.pane_id, PaneId::Terminal(id) if self.editors.contains(id))
                });
                profile_for_client(
                    mode.mode,
                    &clients,
                    resolved_command.as_deref(),
                    editor_active,
                )
            });
        let Some(profile) = profile else {
            self.stop();
            return;
        };
        let pane = clients[0].pane_id;
        if self.focused_pane != Some(pane) || self.profile != Some(profile) {
            self.stop();
            self.focused_pane = Some(pane);
            self.profile = Some(profile);
            self.router = KeyRouter::for_profile(profile);
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

    /// Return the live terminal IDs needed to validate one pair request.
    fn live_ids_for(&self, request: PairRequest) -> HashSet<u32> {
        let candidates = match request {
            PairRequest::Pair(editor, partner) => vec![editor, partner],
            PairRequest::Get(editor) => {
                let mut ids = vec![editor];
                if let Some(partner) = self.pairs.get(editor) {
                    ids.push(partner);
                }
                ids
            }
            PairRequest::Unpair(_) => Vec::new(),
        };
        candidates
            .into_iter()
            .filter(|id| {
                get_pane_info(PaneId::Terminal(*id))
                    .as_ref()
                    .is_some_and(is_pairable_terminal)
            })
            .collect()
    }

    /// Focus a paired editor, or preserve native last-pane behavior.
    fn focus_last_or_pair(&mut self) {
        if let Some(PaneId::Terminal(current)) = self.focused_pane {
            if let Some(partner) = self.pairs.available_partner(current, |id| {
                get_pane_info(PaneId::Terminal(id))
                    .as_ref()
                    .is_some_and(is_pairable_terminal)
            }) {
                focus_pane_with_id(PaneId::Terminal(partner), false, false);
                return;
            }
        }
        focus_last_pane();
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
                NavAction::LastPane => self.focus_last_or_pair(),
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

    /// Give configured Zellij bindings priority, then parse terminal shortcuts.
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
            PermissionType::ReadCliPipes,
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
            Event::PaneUpdate(manifest) => {
                let terminals: HashSet<_> = manifest
                    .panes
                    .values()
                    .flatten()
                    .filter(|pane| !pane.is_plugin)
                    .map(|pane| pane.id)
                    .collect();
                let live: HashSet<_> = manifest
                    .panes
                    .values()
                    .flatten()
                    .filter(|pane| is_pairable_terminal(pane))
                    .map(|pane| pane.id)
                    .collect();
                self.pairs.retain_live(&live);
                self.editors.retain_live(&terminals);
                list_clients();
            }
            Event::TabUpdate(_) => list_clients(),
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

    fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "navi-zellij-pair" && message.name != "navi-zellij-editor" {
            return false;
        }
        let PipeSource::Cli(ref pipe_id) = message.source else {
            return false;
        };
        let Some(payload) = message.payload.as_deref() else {
            return false;
        };
        let result = if message.name == "navi-zellij-pair" {
            parse_request(payload).and_then(|request| {
                let live = self.live_ids_for(request);
                self.pairs.apply(request, &live)
            })
        } else {
            parse_editor_signal(payload).and_then(|signal| {
                let pane_id = match &signal {
                    EditorSignal::Enter { pane_id, .. } | EditorSignal::Leave { pane_id, .. } => {
                        *pane_id
                    }
                };
                if matches!(signal, EditorSignal::Enter { .. })
                    && get_pane_info(PaneId::Terminal(pane_id)).is_none()
                {
                    return Err("pane is no longer available");
                }
                self.editors.apply(signal);
                if self.focused_pane == Some(PaneId::Terminal(pane_id)) {
                    self.stop();
                    list_clients();
                }
                Ok("ok\n".into())
            })
        };
        match result {
            Ok(response) => {
                cli_pipe_output(pipe_id, &response);
                set_cli_pipe_exit_code(pipe_id, 0);
            }
            Err(error) => {
                cli_pipe_output(pipe_id, &format!("error: {error}\n"));
                set_cli_pipe_exit_code(pipe_id, 1);
            }
        }
        false
    }
}

register_plugin!(NaviZellij);
