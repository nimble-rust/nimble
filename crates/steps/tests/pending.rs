/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::types::GameInput;
use nimble_steps::pending_steps::PendingSteps;
use tick_id::TickId;

mod types;

#[test_log::test]
fn add_step() {
    let mut steps = PendingSteps::<GameInput>::new(32, TickId(10));
    let first_tick_id = TickId(12);
    steps
        .set(first_tick_id, GameInput::MoveHorizontal(-2))
        .expect("this should work");
    assert_eq!(steps.front_tick_id(), None);
    assert!(steps.is_empty());
    steps
        .set(first_tick_id - 2, GameInput::Jumping(false))
        .expect("this should work");
    assert!(!steps.is_empty());
    assert_eq!(steps.front_tick_id().unwrap().value(), 10);
    let first_jumping_step = steps.pop();
    assert_eq!(first_jumping_step.tick_id, first_tick_id - 2);
    assert_eq!(steps.front_tick_id(), None);
    steps.discard_up_to(first_tick_id);
    assert!(steps.is_empty());
    steps.discard_up_to(first_tick_id + 1);
    assert!(steps.is_empty());
}
