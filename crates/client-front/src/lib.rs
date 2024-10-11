/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::prelude::{InOctetStream, OutOctetStream};
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use hexify::format_hex;
use log::{debug, trace};
use metricator::{AggregateMetric, MinMaxAvg, RateMetric};
use monotonic_time_rs::{Millis, MillisDuration, MillisLow16};
use nimble_client_stream::client::{ClientStream, ClientStreamError};
pub use nimble_client_stream::ClientLogicPhase;
pub use nimble_client_stream::LocalPlayer;
use nimble_ordered_datagram::{DatagramOrderInError, OrderedIn, OrderedOut};
use nimble_protocol_header::ClientTime;
use nimble_step::Step;
use nimble_step_types::{LocalIndex, StepForParticipants};
use std::fmt::{Debug, Display};
use std::io;
use tick_id::TickId;

#[derive(Debug)]
pub enum ClientFrontError {
    Unexpected(String),
    DatagramOrderInError(DatagramOrderInError),
    ClientStreamError(ClientStreamError),

    IoError(io::Error),
}

impl ErrorLevelProvider for ClientFrontError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::Unexpected(_) => ErrorLevel::Warning,
            Self::DatagramOrderInError(_) => ErrorLevel::Info,
            Self::IoError(_) => ErrorLevel::Warning,
            Self::ClientStreamError(err) => err.error_level(),
        }
    }
}

impl From<DatagramOrderInError> for ClientFrontError {
    fn from(err: DatagramOrderInError) -> ClientFrontError {
        ClientFrontError::DatagramOrderInError(err)
    }
}

impl From<ClientStreamError> for ClientFrontError {
    fn from(err: ClientStreamError) -> ClientFrontError {
        ClientFrontError::ClientStreamError(err)
    }
}

impl From<io::Error> for ClientFrontError {
    fn from(err: io::Error) -> ClientFrontError {
        ClientFrontError::IoError(err)
    }
}

pub struct MetricsInDirection {
    pub datagrams_per_second: f32,
    pub octets_per_second: f32,
}

impl Display for MetricsInDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} datagrams/s {} octets/s",
            self.datagrams_per_second, self.octets_per_second
        )
    }
}

pub struct CombinedMetrics {
    pub outgoing: MetricsInDirection,
    pub incoming: MetricsInDirection,
    pub latency: Option<MinMaxAvg<u16>>,
}

impl Display for CombinedMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let latency_string = self
            .latency
            .as_ref()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not set yet".to_string());

        write!(
            f,
            "metrics: latency:{} out:\n{}, in:\n{}",
            latency_string, self.outgoing, self.incoming
        )
    }
}

pub struct ClientFront<
    StateT: BufferDeserializer + Debug,
    StepT: Clone + Deserialize + Serialize + Debug + Display,
> {
    pub client: ClientStream<StateT, StepT>,
    ordered_datagram_out: OrderedOut,
    ordered_in: OrderedIn,
    latency: AggregateMetric<u16>,
    datagram_drops: AggregateMetric<u16>,
    in_datagrams_per_second: RateMetric,
    in_octets_per_second: RateMetric,
    out_datagrams_per_second: RateMetric,
    out_octets_per_second: RateMetric,

    last_debug_metric_at: Millis,
    debug_metric_duration: MillisDuration,
}

impl<
        StateT: BufferDeserializer + Debug,
        StepT: Clone + Deserialize + Serialize + Debug + Display,
    > ClientFront<StateT, StepT>
{
    pub fn new(deterministic_simulation_version: app_version::Version, now: Millis) -> Self {
        Self {
            client: ClientStream::<StateT, StepT>::new(deterministic_simulation_version),
            ordered_datagram_out: Default::default(),
            ordered_in: Default::default(),
            latency: AggregateMetric::<u16>::new(10).unwrap().with_unit("ms"),
            datagram_drops: AggregateMetric::<u16>::new(10).unwrap(),

            in_datagrams_per_second: RateMetric::with_interval(now, 0.1),
            in_octets_per_second: RateMetric::with_interval(now, 0.1),

            out_datagrams_per_second: RateMetric::with_interval(now, 0.1),
            out_octets_per_second: RateMetric::with_interval(now, 0.1),
            last_debug_metric_at: now,
            debug_metric_duration: MillisDuration::from_secs(1.0).unwrap(),
        }
    }

    pub fn send(&mut self, now: Millis) -> Result<Vec<Vec<u8>>, ClientFrontError> {
        let mut packet = [0u8; 1200];
        let mut out_datagrams: Vec<Vec<u8>> = vec![];

        let datagrams = self.client.send()?;
        for datagram in &datagrams {
            let mut stream = OutOctetStream::new();

            // Serialize
            self.ordered_datagram_out.to_stream(&mut stream)?; // Ordered datagrams
            let client_time = ClientTime::new(now.to_lower());
            client_time.serialize(&mut stream)?;

            packet[0..4].copy_from_slice(stream.octets_ref());
            packet[4..4 + datagram.len()].copy_from_slice(datagram);

            let complete_datagram = packet[0..4 + datagram.len()].to_vec();
            out_datagrams.push(complete_datagram);
            self.ordered_datagram_out.commit();
            self.out_octets_per_second.add(4 + datagram.len() as u32)
        }

        self.out_datagrams_per_second
            .add(out_datagrams.len() as u32);

        Ok(out_datagrams)
    }

    pub fn update(&mut self, now: Millis) {
        self.in_datagrams_per_second.update(now);
        self.in_octets_per_second.update(now);
        self.out_datagrams_per_second.update(now);
        self.out_octets_per_second.update(now);

        if now - self.last_debug_metric_at > self.debug_metric_duration {
            self.last_debug_metric_at = now;
            debug!("metrics: {}", self.metrics())
        }
    }

    pub fn metrics(&self) -> CombinedMetrics {
        CombinedMetrics {
            outgoing: MetricsInDirection {
                datagrams_per_second: self.out_datagrams_per_second(),
                octets_per_second: self.out_octets_per_second(),
            },
            incoming: MetricsInDirection {
                datagrams_per_second: self.in_datagrams_per_second(),
                octets_per_second: self.in_octets_per_second(),
            },
            latency: self.latency(),
        }
    }

    pub fn push_predicted_step(
        &mut self,
        tick_id: TickId,
        step: StepForParticipants<StepT>,
    ) -> Result<(), ClientFrontError> {
        self.client.push_predicted_step(tick_id, step)?;
        Ok(())
    }

    pub fn pop_all_authoritative_steps(
        &mut self,
    ) -> (TickId, Vec<StepForParticipants<Step<StepT>>>) {
        self.client.pop_all_authoritative_steps()
    }

    pub fn receive(&mut self, now: Millis, datagram: &[u8]) -> Result<(), ClientFrontError> {
        trace!("client-front received\n{}", format_hex(datagram));
        let mut in_stream = InOctetStream::new(datagram);
        let dropped_packets = self.ordered_in.read_and_verify(&mut in_stream)?;
        self.datagram_drops.add(dropped_packets.inner());

        self.in_octets_per_second.add(datagram.len() as u32);
        self.in_datagrams_per_second.add(1);

        let client_time = ClientTime::deserialize(&mut in_stream)?;

        let low_16 = client_time.inner() as MillisLow16;

        let earlier = now
            .from_lower(low_16)
            .ok_or_else(|| ClientFrontError::Unexpected("from_lower_error".to_string()))?;
        let duration_ms = now
            .checked_duration_since_ms(earlier)
            .ok_or_else(|| ClientFrontError::Unexpected("earlier".to_string()))?;

        self.latency.add(duration_ms.as_millis() as u16);

        self.client.receive(&datagram[4..])?;

        Ok(())
    }

    pub fn latency(&self) -> Option<MinMaxAvg<u16>> {
        self.latency.values()
    }

    pub fn datagram_drops(&self) -> Option<MinMaxAvg<u16>> {
        self.datagram_drops.values()
    }

    pub fn in_datagrams_per_second(&self) -> f32 {
        self.in_datagrams_per_second.rate()
    }

    pub fn in_octets_per_second(&self) -> f32 {
        self.in_octets_per_second.rate()
    }

    pub fn out_datagrams_per_second(&self) -> f32 {
        self.out_datagrams_per_second.rate()
    }

    pub fn out_octets_per_second(&self) -> f32 {
        self.out_octets_per_second.rate()
    }

    /// Returns the average server buffer delta tick, if available.
    ///
    /// # Returns
    /// An optional average server buffer delta tick.
    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        self.client.server_buffer_delta_ticks()
    }

    pub fn game(&self) -> Option<&StateT> {
        self.client.game()
    }

    pub fn game_mut(&mut self) -> Option<&mut StateT> {
        self.client.game_mut()
    }

    pub fn request_join_player(
        &mut self,
        local_players: Vec<LocalIndex>,
    ) -> Result<(), ClientFrontError> {
        self.client.request_join_player(local_players)?;
        Ok(())
    }
}
