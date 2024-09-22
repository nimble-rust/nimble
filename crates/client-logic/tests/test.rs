/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

/*
#[test_log::test]
#[allow(dead_code)]
fn send_to_host() {

    let random = GetRandom;
    let random_box = Box::new(random);
    let mut client = ClientL::<SampleGame, Step<SampleStep>>::new(random_box);
    let mut udp_client = UdpClient::new("127.0.0.1:23000").unwrap();
    let communicator: &mut dyn DatagramCommunicator = &mut udp_client;
    let random2 = GetRandom;
    let random2_box = Box::new(random2);
    let mut udp_connections_client = udp_connections::Client::new(random2_box);

    let codec: &mut dyn DatagramCodec = &mut udp_connections_client;
    let joining_player = JoinPlayerRequest { local_index: 32 };

    let join_game_request = JoinGameRequest {
        nonce: Nonce(0),
        join_game_type: JoinGameType::NoSecret,
        player_requests: JoinPlayerRequests {
            players: vec![joining_player],
        },
    };
    client
        .set_joining_player(join_game_request)
        .expect("set joining player");
    client.debug_set_tick_id(0x8BADF00D);

    let mut buf = [1u8; 1200];
    for _ in 0..20 {
        let datagrams_to_send = client.send().unwrap();
        for datagram_to_send in datagrams_to_send {
            info!(
                "send nimble datagram of size: {} payload: {}",
                datagram_to_send.len(),
                hex_output(datagram_to_send.as_slice())
            );
            let processed = codec.encode(datagram_to_send.as_slice()).unwrap();
            communicator.send(processed.as_slice()).unwrap();
        }
        if let Ok(size) = communicator.receive(&mut buf) {
            let received_buf = &buf[0..size];
            info!(
                "received datagram of size: {} payload: {}",
                size,
                hex_output(received_buf)
            );
            match codec.decode(received_buf) {
                Ok(datagram_for_client) => {
                    if datagram_for_client.len() > 0 {
                        info!(
                            "received datagram to client: {}",
                            hex_output(&datagram_for_client)
                        );
                        if let Err(e) = client.receive(datagram_for_client.as_slice()) {
                            warn!("receive error {}", e);
                        }
                    }
                }
                Err(some_error) => error!("error {}", some_error),
            }
        }
        thread::sleep(Duration::from_millis(16));
    }
}
*/
