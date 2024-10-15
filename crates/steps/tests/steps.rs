/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::steps_test_types::GameInput;
use nimble_steps::Steps;
use tick_id::TickId;

mod steps_test_types;

#[test_log::test]
fn add_step() {
    let mut steps = Steps::new(TickId(23));
    steps
        .push(TickId(23), GameInput::MoveHorizontal(-2))
        .expect("Expected a move horizontal tick");
    assert_eq!(steps.len(), 1);
    assert_eq!(steps.front_tick_id().unwrap().value(), 23)
}

#[test_log::test]
fn push_and_pop_step() {
    let mut steps = Steps::new(TickId(23));
    steps
        .push(TickId(23), GameInput::Jumping(true))
        .expect("Expected a jumping tick");
    steps
        .push(TickId(24), GameInput::MoveHorizontal(42))
        .expect("Expected a move horizontal tick");
    assert_eq!(steps.len(), 2);
    assert_eq!(steps.front_tick_id().unwrap().value(), 23);
    assert_eq!(steps.pop().unwrap().step, GameInput::Jumping(true));
    assert_eq!(steps.front_tick_id().unwrap().value(), 24);
}

#[test_log::test]
fn push_and_discard_count() {
    let mut steps = Steps::new(TickId(23));
    steps
        .push(TickId(23), GameInput::Jumping(true))
        .expect("Expected a jumping tick");
    steps
        .push(TickId(24), GameInput::MoveHorizontal(42))
        .expect("Expected a move horizontal tick");
    assert_eq!(steps.len(), 2);
    steps.discard_count(8);
    assert_eq!(steps.len(), 0);
}

#[test_log::test]
fn push_and_discard_up_to_lower() {
    let mut steps = Steps::new(TickId(23));
    steps
        .push(TickId(23), GameInput::Jumping(true))
        .expect("Expected a jumping tick");
    steps
        .push(TickId(24), GameInput::MoveHorizontal(42))
        .expect("Expected a move horizontal tick");
    assert_eq!(steps.len(), 2);
    steps.discard_up_to(TickId(1));
    assert_eq!(steps.len(), 2);
}

#[test_log::test]
fn push_and_discard_up_to_equal() {
    let mut steps = Steps::new(TickId(23));
    steps
        .push(TickId(23), GameInput::Jumping(true))
        .expect("Expected a jumping tick");
    steps
        .push(TickId(24), GameInput::MoveHorizontal(42))
        .expect("Expected a move horizontal tick");
    assert_eq!(steps.len(), 2);
    steps.discard_up_to(TickId::new(24));
    assert_eq!(steps.len(), 1);
}
