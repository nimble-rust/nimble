/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use criterion::{criterion_group, criterion_main, Criterion};
use nimble_client_logic::err::ClientError;
use nimble_client_logic::logic::ClientLogic;
use nimble_protocol::host_to_client::{
    AuthoritativeStepRanges, GameStepResponse, GameStepResponseHeader, HostToClientCommands,
};
use nimble_sample_step::{SampleState, SampleStep};

pub fn game_step_response() -> Result<(), ClientError> {
    let mut client_logic = ClientLogic::<SampleState, SampleStep>::new();
    // Create a GameStep command
    let response = GameStepResponse::<SampleStep> {
        response_header: GameStepResponseHeader {
            // We ignore the response for now
            connection_buffer_count: 0,
            delta_buffer: 0,
            last_step_received_from_client: 0,
        },
        authoritative_steps: AuthoritativeStepRanges { ranges: vec![] },
    };
    let command = HostToClientCommands::GameStep(response);

    // Receive
    client_logic.receive(&[command])
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("game_step_response", |b| {
        b.iter(|| game_step_response());
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
