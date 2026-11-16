use navi_zellij::should_intercept;
use zellij_tile::prelude::{ClientInfo, InputMode, PaneId};

fn client(command: &str) -> ClientInfo {
    ClientInfo::new(1, PaneId::Terminal(1), command.into(), true)
}

#[test]
fn only_one_focused_shell_in_locked_mode_is_eligible() {
    for shell in ["zsh", "/usr/bin/zsh", "bash", "fish", "nu", "N/A"] {
        assert!(
            should_intercept(InputMode::Locked, &[client(shell)]),
            "{shell}"
        );
    }
    assert!(!should_intercept(InputMode::Normal, &[client("zsh")]));
    assert!(!should_intercept(InputMode::Locked, &[client("nvim")]));
    assert!(!should_intercept(InputMode::Locked, &[client("unknown")]));
    assert!(!should_intercept(InputMode::Locked, &[]));
    assert!(!should_intercept(
        InputMode::Locked,
        &[client("zsh"), client("zsh")]
    ));
}
