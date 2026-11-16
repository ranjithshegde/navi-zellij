use navi_zellij::{parse_editor_signal, EditorRegistry, EditorSignal};
use std::collections::HashSet;

const OLD: &str = "00000000000000000001:0000000001";
const NEW: &str = "00000000000000000002:0000000002";

fn enter(pane_id: u32, instance: &str, sequence: u64) -> EditorSignal {
    EditorSignal::Enter {
        pane_id,
        instance: instance.into(),
        sequence,
    }
}

fn leave(pane_id: u32, instance: &str, sequence: u64) -> EditorSignal {
    EditorSignal::Leave {
        pane_id,
        instance: instance.into(),
        sequence,
    }
}

#[test]
fn editor_protocol_accepts_zero_pane_and_rejects_malformed_messages() {
    assert_eq!(
        parse_editor_signal("v1 enter 0 00000000000000000001:0000000001 1"),
        Ok(enter(0, OLD, 1))
    );
    assert_eq!(
        parse_editor_signal("v1 leave 2 00000000000000000001:0000000001 2"),
        Ok(leave(2, OLD, 2))
    );
    for bad in [
        "enter 0 pid:1 1",
        "v2 enter 0 pid:1 1",
        "v1 enter -1 pid:1 1",
        "v1 enter 0 pid:1 0",
        "v1 enter 0 pid:1",
        "v1 leave 0 pid:1 2 extra",
        "v1 other 0 pid:1 1",
        "v1 enter 0 pid:1 1",
        "v1 enter 0 00000000000000000001:x 1",
    ] {
        assert!(parse_editor_signal(bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn editor_guard_follows_enter_leave_and_pane_closure() {
    let mut editors = EditorRegistry::default();
    editors.apply(enter(0, OLD, 1));
    assert!(editors.contains(0));
    editors.apply(leave(0, OLD, 2));
    assert!(!editors.contains(0));
    editors.apply(enter(0, OLD, 3));
    assert!(editors.contains(0));
    editors.retain_live(&HashSet::from([2]));
    assert!(!editors.contains(0));
}

#[test]
fn delayed_leave_cannot_cancel_a_later_resume() {
    let mut editors = EditorRegistry::default();
    editors.apply(enter(1, OLD, 1));
    editors.apply(leave(1, OLD, 2));
    editors.apply(enter(1, OLD, 3));
    editors.apply(leave(1, OLD, 2));
    assert!(editors.contains(1));
}

#[test]
fn first_leave_blocks_an_older_enter() {
    let mut editors = EditorRegistry::default();
    editors.apply(leave(1, OLD, 2));
    editors.apply(enter(1, OLD, 1));
    assert!(!editors.contains(1));
    editors.apply(enter(1, OLD, 3));
    assert!(editors.contains(1));
}

#[test]
fn messages_from_an_older_editor_instance_cannot_clear_a_newer_one() {
    let mut editors = EditorRegistry::default();
    editors.apply(enter(1, OLD, 1));
    editors.apply(enter(1, NEW, 1));
    editors.apply(leave(1, OLD, 2));
    editors.apply(enter(1, OLD, 3));
    assert!(editors.contains(1));
    editors.apply(leave(1, NEW, 2));
    assert!(!editors.contains(1));
}

#[test]
fn delayed_older_instance_enter_cannot_replace_newer_editor() {
    let mut editors = EditorRegistry::default();
    editors.apply(enter(1, NEW, 1));
    editors.apply(enter(1, OLD, 1));
    editors.apply(leave(1, OLD, 2));
    assert!(editors.contains(1));
    editors.apply(leave(1, NEW, 2));
    assert!(!editors.contains(1));
}
