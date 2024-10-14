use log::debug;
use metricator::RateMetric;
use monotonic_time_rs::{Millis, MillisDuration};
use std::fmt::Display;

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
}

impl Display for CombinedMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /*let latency_string = self
                    .latency
                    .as_ref()
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "not set yet".to_string());
        */
        write!(
            f,
            "metrics: out:\n{}, in:\n{}",
            self.outgoing, self.incoming
        )
    }
}

pub struct NetworkMetrics {
    in_datagrams_per_second: RateMetric,
    in_octets_per_second: RateMetric,
    out_datagrams_per_second: RateMetric,
    out_octets_per_second: RateMetric,

    last_debug_metric_at: Millis,
    debug_metric_duration: MillisDuration,
}

impl NetworkMetrics {
    pub fn new(now: Millis) -> Self {
        Self {
            in_datagrams_per_second: RateMetric::with_interval(now, 0.1),
            in_octets_per_second: RateMetric::with_interval(now, 0.1),

            out_datagrams_per_second: RateMetric::with_interval(now, 0.1),
            out_octets_per_second: RateMetric::with_interval(now, 0.1),
            last_debug_metric_at: now,
            debug_metric_duration: MillisDuration::from_millis(500),
        }
    }

    pub fn send(&mut self, datagrams: &Vec<Vec<u8>>) {
        for datagram in datagrams {
            self.out_octets_per_second.add(4 + datagram.len() as u32)
        }
        self.out_datagrams_per_second.add(datagrams.len() as u32);
    }

    pub fn receive(&mut self, datagram: &[u8]) {
        self.in_octets_per_second.add(datagram.len() as u32);
        self.in_datagrams_per_second.add(1);
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
                datagrams_per_second: self.out_datagrams_per_second.rate(),
                octets_per_second: self.out_octets_per_second.rate(),
            },
            incoming: MetricsInDirection {
                datagrams_per_second: self.in_datagrams_per_second.rate(),
                octets_per_second: self.in_octets_per_second.rate(),
            },
        }
    }

    /*
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
     */
}
