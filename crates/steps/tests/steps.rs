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

#[test_log::test]
fn iterator_over_steps() {
    let mut steps = Steps::new(TickId::new(0));
    steps.push(TickId::new(0), "Move 1").unwrap();
    steps.push(TickId::new(1), "Move 2").unwrap();
    steps.push(TickId::new(2), "Move 3").unwrap();

    let mut iter = steps.iter();
    assert_eq!(iter.next().unwrap().step, "Move 1");
    assert_eq!(iter.next().unwrap().step, "Move 2");
    assert_eq!(iter.next().unwrap().step, "Move 3");
    assert!(iter.next().is_none());
}

#[test_log::test]
fn iterator_from_index() {
    let mut steps = Steps::default();
    steps.push(TickId::new(0), "Move 1").unwrap();
    steps.push(TickId::new(1), "Move 2").unwrap();
    steps.push(TickId::new(2), "Move 3").unwrap();

    let mut iter = steps.iter_index(1); // Start from index 1 (second item)
    assert_eq!(iter.next().unwrap().step, "Move 2");
    assert_eq!(iter.next().unwrap().step, "Move 3");
    assert!(iter.next().is_none());
}

#[test_log::test]
fn iterator_empty_queue() {
    let steps: Steps<String> = Steps::default();

    let mut iter = steps.iter();
    assert!(iter.next().is_none());
}

#[test_log::test]
fn iterator_from_index_out_of_bounds() {
    let mut steps = Steps::default();
    steps.push(TickId::new(0), "Move 1").unwrap();
    steps.push(TickId::new(1), "Move 2").unwrap();

    let mut iter = steps.iter_index(10); // Start index out of bounds
    assert!(iter.next().is_none()); // No items to iterate over
}

#[test_log::test]
fn into_iter() {
    let mut steps = Steps::default();
    steps.push(TickId::new(0), "Move 1").unwrap();
    steps.push(TickId::new(1), "Move 2").unwrap();
    steps.push(TickId::new(2), "Move 3").unwrap();

    let mut iter = steps.into_iter();
    assert_eq!(iter.next().unwrap().step, "Move 1");
    assert_eq!(iter.next().unwrap().step, "Move 2");
    assert_eq!(iter.next().unwrap().step, "Move 3");
    assert!(iter.next().is_none());
}