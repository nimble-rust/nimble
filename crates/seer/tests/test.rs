/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::*;
use nimble_seer::prelude::*;

use std::io;

pub struct TestGame {
    pub position_x: i32,
}

#[derive(Debug, Clone)]
pub enum TestGameStep {
    MoveLeft,
    MoveRight,
}

impl Deserialize for TestGameStep {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let x = stream.read_u8()?;
        match x {
            0 => Ok(TestGameStep::MoveRight),
            _ => Ok(TestGameStep::MoveLeft),
        }
    }
}

impl Serialize for TestGameStep {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        let v = match self {
            TestGameStep::MoveRight => 0,
            TestGameStep::MoveLeft => 1,
        };
        stream.write_u8(v)
    }
}

impl SeerCallback<TestGameStep> for TestGame {
    fn on_pre_ticks(&mut self) {}

    fn on_tick(&mut self, step: &TestGameStep) {
        match step {
            TestGameStep::MoveLeft => {
                self.position_x -= 1;
            }
            TestGameStep::MoveRight => {
                self.position_x += 1;
            }
        }
    }

    fn on_post_ticks(&mut self) {}
}

#[test_log::test]
fn test_seer() {
    let mut game = TestGame { position_x: -44 };
    let mut seer: Seer<TestGame, TestGameStep> = Seer::new();
    seer.push(TestGameStep::MoveRight);
    seer.update(&mut game);
    assert_eq!(game.position_x, -43);
}
