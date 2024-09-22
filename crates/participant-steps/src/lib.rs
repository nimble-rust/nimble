/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{Deserialize, ReadOctetStream};
use nimble_participant::ParticipantId;
use nimble_steps::Step;
use std::collections::HashMap;
use std::io;

/// `ParticipantSteps` stores the steps for all participants for a single tick of a simulation.
///
/// This struct is generic over `T`, where `T` can be any type that represents a step in the simulation.
/// The steps are stored in a `HashMap` where each participant is identified by a unique `ParticipantId`,
/// and the corresponding value is the step of type `T` that the participant will take during that tick.
///
/// # Usage
/// This struct is typically used in the context of a game or simulation engine where each tick
/// (a discrete time step) requires recording or processing the actions or movements of multiple participants.
///
/// # Examples
/// ```
/// use std::collections::HashMap;
/// use nimble_participant::ParticipantId;
/// use nimble_participant_steps::ParticipantSteps;
/// use nimble_steps::Step;
///
/// struct ExampleStep {
///     action: String
/// }
///
/// let mut steps = HashMap::new();
/// steps.insert(ParticipantId(1), ExampleStep { action: "move".to_string() });
/// steps.insert(ParticipantId(2), ExampleStep { action: "jump".to_string() });
///
/// let participant_steps = ParticipantSteps::<Step<String>>::new();
/// ```
///
/// In this example, `ParticipantSteps` is used to track the actions of two participants in a single tick.
/// Each participant has a unique `ParticipantId` and a `Step` that describes their action for the tick.
#[derive(Clone)]
pub struct ParticipantSteps<T> {
    pub steps: HashMap<ParticipantId, Step<T>>,
}

impl<T> Default for ParticipantSteps<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ParticipantSteps<T> {
    pub fn new() -> Self {
        Self {
            steps: HashMap::new(),
        }
    }

    pub fn insert(&mut self, participant_id: ParticipantId, step: Step<T>) {
        self.steps.insert(participant_id, step);
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl<T: Clone> Deserialize for ParticipantSteps<T> {
    fn deserialize(_: &mut impl ReadOctetStream) -> io::Result<Self>
    where
        Self: Sized,
    {
        todo!()
    }
}
