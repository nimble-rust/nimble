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
    let mut steps = Steps::<GameInput>::new_with_initial_tick(TickId(23));
    steps.push(GameInput::MoveHorizontal(-2));
    assert_eq!(steps.len(), 1);
    assert_eq!(steps.front_tick_id().unwrap().value(), 23)
}

#[test_log::test]
fn push_and_pop_step() {
    let mut steps = Steps::<GameInput>::new_with_initial_tick(TickId(23));
    steps.push(GameInput::Jumping(true));
    steps.push(GameInput::MoveHorizontal(42));
    assert_eq!(steps.len(), 2);
    assert_eq!(steps.front_tick_id().unwrap().value(), 23);
    assert_eq!(steps.pop().unwrap().step, GameInput::Jumping(true));
    assert_eq!(steps.front_tick_id().unwrap().value(), 24);
}

#[test_log::test]
fn push_and_pop_count() {
    let mut steps = Steps::<GameInput>::new_with_initial_tick(TickId(23));
    steps.push(GameInput::Jumping(true));
    steps.push(GameInput::MoveHorizontal(42));
    assert_eq!(steps.len(), 2);
    steps.pop_count(8);
    assert_eq!(steps.len(), 0);
}

#[test_log::test]
fn push_and_pop_up_to_lower() {
    let mut steps = Steps::<GameInput>::new_with_initial_tick(TickId(23));
    steps.push(GameInput::Jumping(true));
    steps.push(GameInput::MoveHorizontal(42));
    assert_eq!(steps.len(), 2);
    steps.pop_up_to(TickId(1));
    assert_eq!(steps.len(), 2);
}

#[test_log::test]
fn push_and_pop_up_to_equal() {
    let mut steps = Steps::<GameInput>::new_with_initial_tick(TickId(23));
    steps.push(GameInput::Jumping(true));
    steps.push(GameInput::MoveHorizontal(42));
    assert_eq!(steps.len(), 2);
    steps.pop_up_to(TickId::new(24));
    assert_eq!(steps.len(), 1);
}
