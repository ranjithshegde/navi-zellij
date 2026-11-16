use navi_zellij::{is_pairable_terminal, parse_request, PairRegistry, PairRequest};
use std::collections::HashSet;
use zellij_tile::prelude::PaneInfo;

#[test]
fn only_active_selectable_terminal_panes_can_be_paired() {
    let pane = PaneInfo {
        is_selectable: true,
        ..PaneInfo::default()
    };
    assert!(is_pairable_terminal(&pane));

    for unavailable in [
        PaneInfo {
            is_plugin: true,
            ..pane.clone()
        },
        PaneInfo {
            exited: true,
            ..pane.clone()
        },
        PaneInfo {
            is_held: true,
            ..pane.clone()
        },
        PaneInfo {
            is_suppressed: true,
            ..pane.clone()
        },
        PaneInfo {
            is_selectable: false,
            ..pane.clone()
        },
    ] {
        assert!(!is_pairable_terminal(&unavailable));
    }
}

#[test]
fn paired_shell_has_a_target_and_unpaired_shell_uses_last_pane() {
    let mut pairs = PairRegistry::default();
    assert_eq!(pairs.available_partner(1, |_| true), None);

    pairs.pair(1, 2).unwrap();
    assert_eq!(pairs.available_partner(1, |id| id == 2), Some(2));
    assert_eq!(pairs.available_partner(2, |id| id == 1), Some(1));

    assert_eq!(pairs.available_partner(1, |_| false), None);
    assert_eq!(
        pairs.get(1),
        None,
        "an unavailable target should expire its pair"
    );
}

#[test]
fn protocol_accepts_only_versioned_terminal_id_requests() {
    assert_eq!(
        parse_request("v1 pair 0 34").unwrap(),
        PairRequest::Pair(0, 34)
    );
    assert_eq!(
        parse_request("v1 pair 12 34").unwrap(),
        PairRequest::Pair(12, 34)
    );
    assert_eq!(parse_request("v1 get 12").unwrap(), PairRequest::Get(12));
    assert_eq!(
        parse_request("v1 unpair 12").unwrap(),
        PairRequest::Unpair(12)
    );
    for bad in [
        "pair 12 34",
        "v2 get 12",
        "v1 get abc",
        "v1 pair 1",
        "v1 get 1 2",
        "v1 pair 2 2",
        "v1 nonsense 1",
    ] {
        assert!(parse_request(bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn replacing_a_partner_removes_the_previous_editors_pair() {
    let mut pairs = PairRegistry::default();
    pairs.pair(1, 2).unwrap();
    pairs.pair(3, 2).unwrap();
    assert_eq!(pairs.get(1), None);
    assert_eq!(pairs.get(2), Some(3));
    assert_eq!(pairs.get(3), Some(2));
}

#[test]
fn first_terminal_can_be_paired() {
    let mut pairs = PairRegistry::default();
    pairs.pair(0, 2).unwrap();
    assert_eq!(pairs.get(0), Some(2));
    assert_eq!(pairs.get(2), Some(0));
}

#[test]
fn unpairing_either_side_removes_both_directions() {
    let mut pairs = PairRegistry::default();
    pairs.pair(1, 2).unwrap();
    pairs.unpair(2);
    assert_eq!(pairs.get(1), None);
    assert_eq!(pairs.get(2), None);
}

#[test]
fn closing_either_pane_expires_the_pair() {
    let mut pairs = PairRegistry::default();
    pairs.pair(1, 2).unwrap();
    pairs.retain_live(&HashSet::from([1]));
    assert_eq!(pairs.get(1), None);
    assert_eq!(pairs.get(2), None);
}

#[test]
fn pipe_requests_reject_closed_panes_and_report_current_pair() {
    let mut pairs = PairRegistry::default();
    let live = HashSet::from([1, 2]);
    assert!(pairs.apply(PairRequest::Pair(1, 3), &live).is_err());
    assert_eq!(pairs.apply(PairRequest::Pair(1, 2), &live).unwrap(), "ok\n");
    assert_eq!(pairs.apply(PairRequest::Get(1), &live).unwrap(), "pane 2\n");
    assert_eq!(pairs.apply(PairRequest::Unpair(1), &live).unwrap(), "ok\n");
    assert_eq!(pairs.apply(PairRequest::Get(1), &live).unwrap(), "none\n");
}
