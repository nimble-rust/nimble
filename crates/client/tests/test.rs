/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use nimble_client::Client;
use nimble_client_front::ClientFrontError;
use nimble_sample_game::SampleGame;
use nimble_sample_step::SampleStep;

#[test_log::test]
fn test() -> Result<(), ClientFrontError> {
    let mut client = Client::<SampleGame, SampleStep>::new();

    client.send()?;

    Ok(())
}
