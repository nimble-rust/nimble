/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_host_logic::combinator::Combinator;
use nimble_participant::ParticipantId;
use nimble_step::Step;
use std::fmt::{Display, Formatter};
use tick_id::TickId;
use tick_queue::Queue;

#[derive(Debug, Clone, PartialEq)]
enum TestStep {
    InGame(i8),
    SelectTeam(u16),
}

impl Display for TestStep {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[test_log::test]
fn combinator_add() {
    let mut combinator = Combinator::<TestStep>::new(TickId(0));
    combinator.create_buffer(ParticipantId(1));
    combinator.create_buffer(ParticipantId(2));

    combinator
        .add(ParticipantId(1), TickId(0), TestStep::InGame(-2))
        .expect("TODO: panic message");
    combinator
        .add(ParticipantId(2), TickId(0), TestStep::SelectTeam(42))
        .expect("TODO: panic message");

    assert_eq!(combinator.in_buffers.len(), 2);
    assert_eq!(
        combinator.in_buffers.get(&ParticipantId(1)).unwrap().len(),
        1
    );
    let steps_for_participant_1: &mut Queue<TestStep> =
        combinator.in_buffers.get_mut(&ParticipantId(1)).unwrap();
    let first_step_for_participant_1 = steps_for_participant_1.pop().unwrap();
    assert_eq!(first_step_for_participant_1.item, TestStep::InGame(-2));

    assert_eq!(
        combinator.in_buffers.get(&ParticipantId(2)).unwrap().len(),
        1
    );

    let (produced_tick_id, combined_step) = combinator.produce().unwrap();

    assert_eq!(produced_tick_id, TickId(0));
    assert_eq!(combined_step.len(), 1);
    let first_step = combined_step.get(&ParticipantId(2)); // Participant 1 has been popped up previously
    assert_eq!(first_step.unwrap(), &Step::Custom(TestStep::SelectTeam(42)));
}
