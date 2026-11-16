use navi_zellij::{needs_command_lookup, profile_for_client, InputProfile};
use zellij_tile::prelude::{ClientInfo, InputMode, PaneId};

fn client(command: &str) -> ClientInfo {
    ClientInfo::new(1, PaneId::Terminal(1), command.into(), true)
}

#[test]
fn shell_and_unlabelled_panes_need_a_fresh_command() {
    assert!(needs_command_lookup("zsh"));
    assert!(needs_command_lookup("-zsh"));
    assert!(needs_command_lookup("/usr/bin/bash"));
    assert!(needs_command_lookup("N/A"));
    assert!(!needs_command_lookup("claude"));

    for label in ["zsh", "N/A"] {
        let clients = [client(label)];
        assert_eq!(
            profile_for_client(InputMode::Locked, &clients, None, false),
            None
        );
        assert_eq!(
            profile_for_client(
                InputMode::Locked,
                &clients,
                Some(&["/usr/bin/fish".into()]),
                false,
            ),
            Some(InputProfile::Shell),
        );
        assert_eq!(
            profile_for_client(
                InputMode::Locked,
                &clients,
                Some(&["/opt/claude-code/bin/claude".into()]),
                false,
            ),
            Some(InputProfile::App),
        );
    }
    assert_eq!(
        profile_for_client(
            InputMode::Locked,
            &[client("N/A")],
            Some(&["-zsh".into()]),
            false
        ),
        Some(InputProfile::Shell),
    );
    assert_eq!(
        profile_for_client(
            InputMode::Locked,
            &[client("N/A")],
            Some(&["/opt/My Apps/nvim".into()]),
            false,
        ),
        None,
        "resolved argv[0] is a path, even when it contains spaces",
    );
}

#[test]
fn named_terminal_apps_are_intercepted_except_neovim() {
    for app in ["claude", "/usr/bin/less", "btop"] {
        assert_eq!(
            profile_for_client(InputMode::Locked, &[client(app)], None, false),
            Some(InputProfile::App),
            "{app}",
        );
    }
    for editor in ["nvim", "/usr/bin/nvim --clean"] {
        assert_eq!(
            profile_for_client(InputMode::Locked, &[client(editor)], None, false),
            None,
            "{editor}",
        );
    }
    assert_eq!(
        profile_for_client(InputMode::Locked, &[client("claude")], None, true),
        None,
        "the editor signal overrides a stale app label",
    );
}

#[test]
fn interception_requires_locked_mode_and_one_current_terminal_client() {
    assert_eq!(
        profile_for_client(InputMode::Normal, &[client("claude")], None, false),
        None,
    );
    assert_eq!(
        profile_for_client(InputMode::Locked, &[], None, false),
        None
    );
    assert_eq!(
        profile_for_client(
            InputMode::Locked,
            &[client("zsh"), client("zsh")],
            None,
            false
        ),
        None,
    );
    let mut unfocused = client("claude");
    unfocused.is_current_client = false;
    assert_eq!(
        profile_for_client(InputMode::Locked, &[unfocused], None, false),
        None,
    );
    let mut plugin = client("claude");
    plugin.pane_id = PaneId::Plugin(1);
    assert_eq!(
        profile_for_client(InputMode::Locked, &[plugin], None, false),
        None,
    );
}
