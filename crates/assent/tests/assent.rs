/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_assent::prelude::*;
use std::fmt::Display;

pub struct TestGame {
    pub position_x: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum TestGameStep {
    MoveLeft,
    MoveRight,
}

impl Display for TestGameStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl AssentCallback<TestGameStep> for TestGame {
    fn on_tick(&mut self, step: &TestGameStep) {
        match step {
            TestGameStep::MoveLeft => self.position_x -= 1,
            TestGameStep::MoveRight => self.position_x += 1,
        }
    }
}

#[test_log::test]
fn one_step() {
    let mut game = TestGame { position_x: -44 };
    let mut assent: Assent<TestGame, TestGameStep> = Assent::default();
    let step = TestGameStep::MoveLeft;
    assent.push(step);
    assert_eq!(assent.update(&mut game), UpdateState::ConsumedAllKnowledge);
    assert_eq!(game.position_x, -45);
}

#[test_log::test]
fn multiple_steps() {
    let mut game = TestGame { position_x: -44 };
    let mut assent: Assent<TestGame, TestGameStep> = Assent::default();
    let step = TestGameStep::MoveRight;
    assent.push(step);
    assent.push(step);
    assert_eq!(assent.update(&mut game), UpdateState::ConsumedAllKnowledge);
    assert_eq!(game.position_x, -42);
}
