use zellij_tile::prelude::{ClientInfo, InputMode, PaneId};

/// Return whether shell key interception is safe for the current client state.
///
/// Interception requires Locked mode, exactly one client, and a focused shell
/// terminal. A fresh command lookup resolves Zellij's `N/A` default-shell label.
/// If that lookup fails, input is left to Zellij unchanged.
pub fn should_intercept(
    mode: InputMode,
    clients: &[ClientInfo],
    resolved_command: Option<&[String]>,
) -> bool {
    if mode != InputMode::Locked || clients.len() != 1 {
        return false;
    }
    let client = &clients[0];
    let command = if client.running_command == "N/A" {
        resolved_command
            .and_then(|args| args.first())
            .map(String::as_str)
    } else {
        Some(client.running_command.as_str())
    };
    client.is_current_client
        && matches!(client.pane_id, PaneId::Terminal(_))
        && command.is_some_and(is_shell)
}

fn is_shell(command: &str) -> bool {
    let executable = command.split_whitespace().next().unwrap_or("");
    let executable = executable.rsplit('/').next().unwrap_or("");
    matches!(
        executable,
        "sh" | "bash" | "zsh" | "fish" | "nu" | "dash" | "ksh"
    )
}
