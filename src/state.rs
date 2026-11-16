use zellij_tile::prelude::{ClientInfo, InputMode, PaneId};

/// Return whether shell key interception is safe for the current client state.
///
/// Interception requires Locked mode, exactly one client, and a focused shell
/// terminal. Unknown foreground commands are left to Zellij unchanged.
pub fn should_intercept(mode: InputMode, clients: &[ClientInfo]) -> bool {
    if mode != InputMode::Locked || clients.len() != 1 {
        return false;
    }
    let client = &clients[0];
    client.is_current_client
        && matches!(client.pane_id, PaneId::Terminal(_))
        && is_shell(&client.running_command)
}

fn is_shell(command: &str) -> bool {
    // Zellij reports its pane's default shell as N/A when no child job is running.
    if command == "N/A" {
        return true;
    }
    let executable = command.split_whitespace().next().unwrap_or("");
    let executable = executable.rsplit('/').next().unwrap_or("");
    matches!(
        executable,
        "sh" | "bash" | "zsh" | "fish" | "nu" | "dash" | "ksh"
    )
}
