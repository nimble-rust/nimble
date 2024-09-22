/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod client;
mod datagram_build;
mod datagram_parse;
/*
    pub fn send(&mut self) -> io::Result<Vec<Vec<u8>>> {
        let mut out_stream = OutOctetStream::new();
        self.write_header(&mut out_stream)?;

        match self.phase {
            ClientPhase::Connecting(nonce) => {
                let connect_cmd = ConnectRequest {
                    nimble_version: Version {
                        major: 0,
                        minor: 0,
                        patch: 5,
                    },
                    use_debug_stream: false,
                    application_version: Version {
                        major: 1,
                        minor: 0,
                        patch: 0,
                    },
                    nonce,
                };

                ClientToHostOobCommands::ConnectType(connect_cmd).to_stream(&mut out_stream)?;
            }
            ClientPhase::Connected(connection_id, seed) => {
                let client_commands_to_send = self.logic.as_mut().expect("reason").send();
                for command_to_send in client_commands_to_send.iter() {
                    info!("sending command {:?}", command_to_send);
                    command_to_send.to_stream(&mut out_stream)?;
                }
                info!("writing connected header");
                self.write_to_start_of_header(connection_id, seed, &mut out_stream)?
            }
        }


        pub struct Client<
    Game: SeerCallback<AuthoritativeStep<StepT>>
    + AssentCallback<AuthoritativeStep<StepT>>
    + RectifyCallback,
    StepT: Clone + Deserialize + Serialize + Debug,
> {
    logic: Option<ClientLogic<Game, StepT>>,
    tick_id: u32,
    debug_tick_id_to_send: u32,
}

impl<
    Game: SeerCallback<AuthoritativeStep<StepT>>
    + AssentCallback<AuthoritativeStep<StepT>>
    + RectifyCallback,
    StepT: Clone + Deserialize + Serialize + Debug,
> Client<Game, StepT>
{
    pub fn new(mut random: Box<dyn SecureRandom>) -> Client<Game, StepT> {
        let nonce = Nonce(random.get_random_u64());
        Self {
            //random,
            logic: None,
            tick_id: 0,
            debug_tick_id_to_send: 0,
        }
    }

    pub fn set_joining_player(&mut self, join_game_request: JoinGameRequest) -> Result<(), String> {
        self.logic
            .as_mut()
            .ok_or("Logic is not initialized".to_string())
            .map(|logic| logic.set_joining_player(join_game_request))
    }

    pub fn debug_set_tick_id(&mut self, tick_id: u32) {
        self.tick_id = tick_id;
        self.debug_tick_id_to_send = self.tick_id;
    }

    pub fn update(&mut self, game: &mut Game) -> Result<(), String> {
        self.logic
            .as_mut()
            .ok_or("Logic is not initialized".to_string())
            .map(|logic| logic.update(game))
    }

    pub fn add_predicted_step(&mut self, step: PredictedStep<StepT>) -> Result<(), String> {
        self.logic
            .as_mut()
            .ok_or("Logic is not initialized".to_string())
            .map(|logic| logic.add_predicted_step(step))
    }


    pub fn send(&mut self) -> io::Result<Vec<Vec<u8>>> {
                let client_commands_to_send = self.logic.as_mut().expect("reason").send();
                for command_to_send in client_commands_to_send.iter() {
                    info!("sending command {:?}", command_to_send);
                    command_to_send.to_stream(&mut out_stream)?;
                }
                info!("writing connected header");
                self.write_to_start_of_header(connection_id, seed, &mut out_stream)?
            }
    }

    fn on_connect(&mut self, cmd: ConnectionAccepted) -> io::Result<()> {
        match self.phase {
            ClientPhase::Connecting(_) => {
                let half_secret = cmd.host_assigned_connection_secret.value as u32;
                info!("half_secret: {:X}", half_secret);
                self.phase = ClientPhase::Connected(
                    cmd.host_assigned_connection_id,
                    ConnectionSecretSeed(half_secret),
                );

                /*
                ClientToHostCommands::JoinGameType(JoinGameRequest {
                    nonce: Nonce(self.random.get_random_u64()),
                    join_game_type: JoinGameType::NoSecret,
                    player_requests: JoinPlayerRequests {
                        players: vec![JoinPlayerRequest { local_index: 42 }],
                    },
                });

                 */
                info!("connected! cmd:{:?}", cmd);
                Ok(())
            }
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "can not receive on_connect in current client state '{:?}'",
                    self.phase
                ),
            )),
        }
    }

    pub fn receive(&mut self, datagram: &[u8]) -> io::Result<()> {
        let mut in_stream = InOctetStream::new(datagram);
        let connection_mode = ConnectionLayerMode::from_stream(&mut in_stream)?;
        match connection_mode {
            ConnectionLayerMode::OOB => {
                let command = HostToClientOobCommands::from_stream(&mut in_stream)?;
            }
            ConnectionLayerMode::Connection(connection_layer) => {
                match self.phase {
                    ClientPhase::Connecting(_) => Err(Error::new(
                        ErrorKind::InvalidData,
                        "received connection layer connection mode without a connection",
                    )),
                    ClientPhase::Connected(connection_id, connection_seed) => {
                        if connection_layer.connection_id != connection_id {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "wrong connection id, expected {:?} but received {:?}",
                                    connection_id, connection_layer.connection_id
                                ),
                            ));
                        }

                        verify_hash(
                            connection_layer.murmur3_hash,
                            connection_seed,
                            &datagram[5..],
                        )?;

                        self.ordered_datagram_in.read_and_verify(&mut in_stream)?;
                        let _ = client_in_ping(&mut in_stream)?;

                        // TODO: Add latency calculations

                        let mut commands = vec![];
                        for _ in 0..8 {
                            // only allowed to have at maximum eight commands in one datagram
                            if in_stream.has_reached_end() {
                                break;
                            }
                            let command = HostToClientCommands::from_stream(&mut in_stream)?;
                            commands.push(command);
                        }

                        self.logic
                            .as_mut()
                            .expect("reason")
                            .receive(commands.as_slice())
                            .map_err(|err| {
                                Error::new(
                                    ErrorKind::InvalidData,
                                    format!("problem in client: {:?}", err),
                                )
                            })?;

                        Ok(())
                    }
                }
            }
        }
    }
}

 */
