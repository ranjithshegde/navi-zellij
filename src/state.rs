use zellij_tile::prelude::{ClientInfo, InputMode, PaneId};

/// Which shortcut grammar applies to the focused terminal application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProfile {
    /// A shell accepts window and tab shortcuts.
    Shell,
    /// Another terminal app accepts only the `Ctrl-w` window commands.
    App,
}

/// Shell and unlabelled panes need a fresh foreground-command lookup.
pub fn needs_command_lookup(label: &str) -> bool {
    label == "N/A" || is_shell(label)
}

/// Select shortcuts only for one current terminal client in Locked mode.
///
/// `resolved_command` is required when Zellij labels the client as a shell or
/// `N/A`; a stale shell label must not classify a foreground app as a shell.
pub fn profile_for_client(
    mode: InputMode,
    clients: &[ClientInfo],
    resolved_command: Option<&[String]>,
    editor_active: bool,
) -> Option<InputProfile> {
    if mode != InputMode::Locked || clients.len() != 1 || editor_active {
        return None;
    }
    let client = &clients[0];
    if !client.is_current_client || !matches!(client.pane_id, PaneId::Terminal(_)) {
        return None;
    }
    let executable = if needs_command_lookup(&client.running_command) {
        executable_name(resolved_command?.first()?)?
    } else {
        label_executable(&client.running_command)?
    };
    if executable == "nvim" {
        None
    } else if is_shell_executable(executable) {
        Some(InputProfile::Shell)
    } else {
        Some(InputProfile::App)
    }
}

fn is_shell(command: &str) -> bool {
    label_executable(command).is_some_and(is_shell_executable)
}

fn is_shell_executable(executable: &str) -> bool {
    matches!(
        executable,
        "sh" | "bash" | "zsh" | "fish" | "nu" | "dash" | "ksh"
    )
}

fn label_executable(label: &str) -> Option<&str> {
    executable_name(label.split_whitespace().next()?)
}

fn executable_name(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next()?;
    let name = name.strip_prefix('-').unwrap_or(name);
    (!name.is_empty()).then_some(name)
}
