use navi_zellij::{Direction, Effect, KeyRouter, NavAction, ResizeAction};
use zellij_tile::prelude::KeyWithModifier;

fn key(value: &str) -> KeyWithModifier {
    value.parse().unwrap()
}

#[test]
fn ctrl_w_h_moves_left_without_writing_to_shell() {
    let mut router = KeyRouter::default();
    assert!(router.handle(key("Ctrl w")).is_empty());
    assert_eq!(
        router.handle(key("h")),
        vec![Effect::Action(NavAction::Focus(Direction::Left))]
    );
}

#[test]
fn ctrl_w_window_commands_dispatch_the_expected_actions() {
    let cases = [
        ("j", NavAction::Focus(Direction::Down)),
        ("k", NavAction::Focus(Direction::Up)),
        ("l", NavAction::Focus(Direction::Right)),
        ("s", NavAction::Split(Direction::Down)),
        ("v", NavAction::Split(Direction::Right)),
        ("c", NavAction::Close),
        ("w", NavAction::NextPane),
        ("Shift w", NavAction::PreviousPane),
        ("p", NavAction::LastPane),
        ("+", NavAction::Resize(ResizeAction::Grow)),
        ("-", NavAction::Resize(ResizeAction::Shrink)),
        (">", NavAction::Resize(ResizeAction::GrowRight)),
        ("<", NavAction::Resize(ResizeAction::ShrinkRight)),
    ];
    for (suffix, expected) in cases {
        let mut router = KeyRouter::default();
        assert!(router.handle(key("Ctrl w")).is_empty());
        assert_eq!(
            router.handle(key(suffix)),
            vec![Effect::Action(expected)],
            "{suffix}"
        );
    }
}

#[test]
fn shell_tab_keys_and_literal_escape() {
    let mut router = KeyRouter::default();
    assert!(router.handle(key("g")).is_empty());
    assert_eq!(
        router.handle(key("t")),
        vec![Effect::Action(NavAction::NextTab)]
    );
    assert!(router.handle(key("g")).is_empty());
    assert_eq!(
        router.handle(key("Shift t")),
        vec![Effect::Action(NavAction::PreviousTab)]
    );

    assert_eq!(
        router.handle(key("Ctrl v")),
        vec![Effect::Forward(key("Ctrl v"))]
    );
    assert_eq!(router.handle(key("g")), vec![Effect::Forward(key("g"))]);
    assert_eq!(router.handle(key("t")), vec![Effect::Forward(key("t"))]);
}

#[test]
fn incomplete_and_unknown_prefixes_reach_the_shell_in_order() {
    let mut router = KeyRouter::default();
    assert!(router.handle(key("g")).is_empty());
    assert_eq!(router.timeout(), vec![Effect::Forward(key("g"))]);
    assert!(router.handle(key("Ctrl w")).is_empty());
    assert_eq!(router.timeout(), vec![Effect::Forward(key("Ctrl w"))]);
    assert!(router.handle(key("g")).is_empty());
    assert_eq!(
        router.handle(key("i")),
        vec![Effect::Forward(key("g")), Effect::Forward(key("i"))]
    );
    assert!(router.handle(key("Ctrl w")).is_empty());
    assert_eq!(
        router.handle(key("x")),
        vec![Effect::Forward(key("Ctrl w")), Effect::Forward(key("x"))]
    );
}
