/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::*;
use log::info;
use nimble_assent::AssentCallback;
use nimble_participant::ParticipantId;
use nimble_participant_steps::ParticipantSteps;
use nimble_rectify::{Rectify, RectifyCallback};
use nimble_seer::SeerCallback;
use nimble_steps::Step;
use nimble_steps::Step::Custom;
use std::io;

#[derive(Clone)]
pub struct TestGame {
    pub position_x: i32,
}

impl TestGame {
    pub fn on_tick(&mut self, steps: &ParticipantSteps<TestGameStep>) {
        info!("sim tick!");
        for (_, step) in steps.steps.iter() {
            match step {
                Custom(TestGameStep::MoveLeft) => self.position_x -= 1,
                Custom(TestGameStep::MoveRight) => self.position_x += 1,
                Step::Forced => todo!(),
                Step::WaitingForReconnect => todo!(),
                Step::Joined(_) => todo!(),
                Step::Left => todo!(),
            }
        }
    }
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

pub struct CombinedGame {
    pub authoritative_game: TestGame,
    pub predicted_game: TestGame,
}

impl RectifyCallback for CombinedGame {
    fn on_copy_from_authoritative(&mut self) {
        info!("on_copy_from_authoritative");
        self.predicted_game = self.authoritative_game.clone();
    }
}

impl SeerCallback<ParticipantSteps<TestGameStep>> for CombinedGame {
    fn on_tick(&mut self, combined_step: &ParticipantSteps<TestGameStep>) {
        info!("predict tick!");

        self.predicted_game.on_tick(combined_step);
    }
}

impl AssentCallback<ParticipantSteps<TestGameStep>> for CombinedGame {
    fn on_tick(&mut self, combined_step: &ParticipantSteps<TestGameStep>) {
        info!("authoritative tick!");
        self.authoritative_game.on_tick(combined_step);
    }
}

#[test_log::test]
fn one_prediction() {
    let authoritative_game = TestGame { position_x: -44 };
    let predicted_game = TestGame { position_x: -44 };

    let mut callbacks = CombinedGame {
        authoritative_game,
        predicted_game,
    };

    let mut rectify = Rectify::<CombinedGame, ParticipantSteps<TestGameStep>>::new();
    let mut participant_step_combined = ParticipantSteps::<TestGameStep>::new();
    participant_step_combined.insert(ParticipantId(0), Custom(TestGameStep::MoveLeft));

    rectify.push_predicted(participant_step_combined);

    rectify.update(&mut callbacks);

    assert_eq!(callbacks.authoritative_game.position_x, -44);
    assert_eq!(callbacks.predicted_game.position_x, -45);
}

#[test_log::test]
fn one_authoritative_and_one_prediction() {
    let authoritative_game = TestGame { position_x: -44 };
    let predicted_game = TestGame { position_x: -44 };

    let mut callbacks = CombinedGame {
        authoritative_game,
        predicted_game,
    };

    let mut rectify = Rectify::<CombinedGame, ParticipantSteps<TestGameStep>>::new();

    let mut authoritative_step_combined = ParticipantSteps::<TestGameStep>::new();
    authoritative_step_combined.insert(ParticipantId(0), Custom(TestGameStep::MoveRight));
    rectify.push_authoritative(authoritative_step_combined);

    let mut predicted_step_combined = ParticipantSteps::<TestGameStep>::new();
    predicted_step_combined.insert(ParticipantId(0), Custom(TestGameStep::MoveLeft));

    rectify.push_predicted(predicted_step_combined);
    rectify.update(&mut callbacks);

    assert_eq!(callbacks.authoritative_game.position_x, -43);
    assert_eq!(callbacks.predicted_game.position_x, -44);
}

#[test_log::test]
fn one_authoritative_and_x_predictions() {
    let authoritative_game = TestGame { position_x: -44 };
    let predicted_game = TestGame { position_x: -44 };

    let mut callbacks = CombinedGame {
        authoritative_game,
        predicted_game,
    };

    let mut rectify = Rectify::<CombinedGame, ParticipantSteps<TestGameStep>>::new();

    let mut authoritative_step_combined = ParticipantSteps::<TestGameStep>::new();
    authoritative_step_combined.insert(ParticipantId(0), Custom(TestGameStep::MoveRight));
    rectify.push_authoritative(authoritative_step_combined);

    let mut predicted_step_combined = ParticipantSteps::<TestGameStep>::new();
    predicted_step_combined.insert(ParticipantId(0), Custom(TestGameStep::MoveLeft));

    rectify.push_predicted(predicted_step_combined.clone());
    rectify.push_predicted(predicted_step_combined.clone());
    rectify.push_predicted(predicted_step_combined.clone());
    rectify.update(&mut callbacks);

    assert_eq!(callbacks.authoritative_game.position_x, -43);
    assert_eq!(callbacks.predicted_game.position_x, -45);
}
