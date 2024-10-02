/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::{InOctetStream, OutOctetStream};
use flood_rs::{BufferDeserializer, Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use log::info;
use nimble_assent::{AssentCallback, DeterministicVersion, DeterministicVersionProvider};
use nimble_rectify::RectifyCallback;
use nimble_seer::SeerCallback;
use nimble_step_types::AuthoritativeStep;
use nimble_steps::Step;
use std::io;

pub use nimble_sample_step::SampleStep;

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct SampleGameState {
    pub x: i32,
    pub y: i32,
}

impl SampleGameState {
    pub fn update(&mut self, step: &AuthoritativeStep<Step<SampleStep>>) {
        for (participant_id, step) in &step.authoritative_participants {
            match &step {
                Step::Custom(custom) => match custom {
                    SampleStep::MoveLeft(amount) => self.x -= *amount as i32,
                    SampleStep::MoveRight(amount) => self.x += *amount as i32,
                    SampleStep::Jump => self.y += 1,
                    SampleStep::Nothing => {}
                },
                Step::Forced => self.y += 1,
                Step::WaitingForReconnect => info!("waiting for reconnect"),
                Step::Joined(data) => info!(
                    "participant {} joined at time {}",
                    participant_id, data.tick_id
                ),
                Step::Left => info!("participant {} left", participant_id),
            }
        }
    }

    pub fn to_octets(&self) -> io::Result<Vec<u8>> {
        let mut out = OutOctetStream::new();
        out.write_i32(self.x)?;
        out.write_i32(self.y)?;
        Ok(out.octets())
    }

    #[allow(unused)]
    pub fn from_octets(payload: &[u8]) -> io::Result<Self> {
        let mut in_stream = InOctetStream::new(payload);
        Ok(Self {
            x: in_stream.read_i32()?,
            y: in_stream.read_i32()?,
        })
    }
}

impl BufferDeserializer for SampleGameState {
    fn deserialize(buf: &[u8]) -> io::Result<(Self, usize)> {
        let mut in_stream = InOctetStream::new(buf);
        let s = <SampleGameState as Deserialize>::deserialize(&mut in_stream)?;
        Ok((s, in_stream.cursor.position() as usize))
    }
}

impl Serialize for SampleGameState {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_i32(self.x)?;
        stream.write_i32(self.y)
    }
}

impl Deserialize for SampleGameState {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            x: stream.read_i32()?,
            y: stream.read_i32()?,
        })
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct SampleGame {
    pub predicted: SampleGameState,
    pub authoritative: SampleGameState,
}

impl SampleGame {
    pub fn authoritative_octets(&self) -> io::Result<Vec<u8>> {
        self.authoritative.to_octets()
    }
}

impl BufferDeserializer for SampleGame {
    fn deserialize(_: &[u8]) -> io::Result<(Self, usize)> {
        todo!()
    }
}

impl DeterministicVersionProvider for SampleGame {
    fn deterministic_version() -> DeterministicVersion {
        DeterministicVersion {
            major: 0,
            minor: 0,
            patch: 5,
        }
    }
}

impl Serialize for SampleGame {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.authoritative.serialize(stream)
    }
}

impl Deserialize for SampleGame {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            authoritative: <SampleGameState as Deserialize>::deserialize(stream)?,
            predicted: SampleGameState::default(),
        })
    }
}

impl SeerCallback<AuthoritativeStep<Step<SampleStep>>> for SampleGame {
    fn on_tick(&mut self, step: &AuthoritativeStep<Step<SampleStep>>) {
        self.predicted.update(step);
    }
}

impl AssentCallback<AuthoritativeStep<Step<SampleStep>>> for SampleGame {
    fn on_pre_ticks(&mut self) {
        self.predicted = self.authoritative.clone();
    }
    fn on_tick(&mut self, step: &AuthoritativeStep<Step<SampleStep>>) {
        self.authoritative.update(step);
    }
    fn on_post_ticks(&mut self) {
        self.authoritative = self.predicted.clone();
    }
}

impl RectifyCallback for SampleGame {
    fn on_copy_from_authoritative(&mut self) {
        self.predicted = self.authoritative.clone();
    }
}
