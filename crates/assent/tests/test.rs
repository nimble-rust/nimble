/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_assent::prelude::*;

pub struct TestGame {
    pub position_x: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum TestGameStep {
    MoveLeft,
    MoveRight,
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
fn test_assent() {
    let mut game = TestGame { position_x: -44 };
    let mut assent: Assent<TestGame, TestGameStep> = Assent::new();
    let step = TestGameStep::MoveLeft;
    assent.push(step);
    assent.update(&mut game);
    assert_eq!(game.position_x, -45);
}

#[test_log::test]
fn test_assent_right() {
    let mut game = TestGame { position_x: -44 };
    let mut assent: Assent<TestGame, TestGameStep> = Assent::new();
    let step = TestGameStep::MoveRight;
    assent.push(step);
    assent.push(step);
    assent.update(&mut game);
    assert_eq!(game.position_x, -42);
}
