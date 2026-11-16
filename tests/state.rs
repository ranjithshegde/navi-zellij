use navi_zellij::should_intercept;
use zellij_tile::prelude::{ClientInfo, InputMode, PaneId};

fn client(command: &str) -> ClientInfo {
    ClientInfo::new(1, PaneId::Terminal(1), command.into(), true)
}

#[test]
fn only_one_focused_shell_in_locked_mode_is_eligible() {
    for shell in ["zsh", "/usr/bin/zsh", "bash", "fish", "nu"] {
        assert!(
            should_intercept(InputMode::Locked, &[client(shell)], None),
            "{shell}"
        );
    }
    assert!(!should_intercept(InputMode::Normal, &[client("zsh")], None));
    assert!(!should_intercept(
        InputMode::Locked,
        &[client("nvim")],
        None
    ));
    assert!(!should_intercept(
        InputMode::Locked,
        &[client("unknown")],
        None
    ));
    assert!(!should_intercept(InputMode::Locked, &[client("N/A")], None));
    assert!(!should_intercept(InputMode::Locked, &[], None));
    assert!(!should_intercept(
        InputMode::Locked,
        &[client("zsh"), client("zsh")],
        None
    ));
}

#[test]
fn default_shell_requires_a_fresh_command_lookup() {
    let clients = [client("N/A")];
    assert!(should_intercept(
        InputMode::Locked,
        &clients,
        Some(&["/usr/bin/zsh".to_owned()])
    ));
    assert!(!should_intercept(InputMode::Locked, &clients, None));
    assert!(!should_intercept(
        InputMode::Locked,
        &clients,
        Some(&["nvim".to_owned()])
    ));
}
