/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_host_logic::combinator::Combinator;
use nimble_participant::ParticipantId;
use nimble_steps::{Step, Steps};
use tick_id::TickId;

#[derive(Debug, Clone, PartialEq)]
enum TestStep {
    InGame(i8),
    SelectTeam(u16),
}

#[test_log::test]
fn combinator_add() {
    let mut combinator = Combinator::<TestStep>::new(TickId(0));
    combinator.create_buffer(ParticipantId(1));
    combinator.create_buffer(ParticipantId(2));

    combinator
        .add(ParticipantId(1), TestStep::InGame(-2))
        .expect("TODO: panic message");
    combinator
        .add(ParticipantId(2), TestStep::SelectTeam(42))
        .expect("TODO: panic message");

    assert_eq!(combinator.in_buffers.len(), 2);
    assert_eq!(
        combinator.in_buffers.get(&ParticipantId(1)).unwrap().len(),
        1
    );
    let steps_for_participant_1: &mut Steps<TestStep> =
        combinator.in_buffers.get_mut(&ParticipantId(1)).unwrap();
    let first_step_for_participant_1 = steps_for_participant_1.pop().unwrap();
    assert_eq!(first_step_for_participant_1.step, TestStep::InGame(-2));

    assert_eq!(
        combinator.in_buffers.get(&ParticipantId(2)).unwrap().len(),
        1
    );

    let combined_step = combinator.produce().unwrap();

    assert_eq!(combined_step.combined_step.len(), 1);
    let first_step = combined_step.combined_step.get(&ParticipantId(2)); // Participant 1 has been popped up previously
    assert_eq!(first_step.unwrap(), &Step::Custom(TestStep::SelectTeam(42)));
}
