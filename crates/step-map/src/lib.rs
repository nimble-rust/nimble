/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_participant::ParticipantId;
use seq_map::SeqMap;

pub type StepMap<StepT> = SeqMap<ParticipantId, StepT>;
