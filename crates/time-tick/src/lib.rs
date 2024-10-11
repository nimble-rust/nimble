use log::trace;
use monotonic_time_rs::{Millis, MillisDuration};

#[derive(Debug)]
pub struct TimeTick {
    tick_time_duration: MillisDuration,
    consumed_absolute_time: Millis,
    max_tick_per_update: u16,
}

impl TimeTick {
    pub const fn new(now: Millis, factor_milli: MillisDuration, max_tick_per_update: u16) -> Self {
        TimeTick {
            tick_time_duration: factor_milli,
            consumed_absolute_time: now,
            max_tick_per_update,
        }
    }

    pub fn set_time_period(&mut self, factor_milli: MillisDuration) {
        self.tick_time_duration = factor_milli;
    }

    pub fn reset(&mut self, now: Millis) {
        self.consumed_absolute_time = now;
    }

    #[inline]
    pub fn update(&mut self, now: Millis) -> u16 {
        let time_ahead = now - self.consumed_absolute_time;
        let tick_count = (time_ahead.as_millis() / self.tick_time_duration.as_millis()) as u16;
        trace!("time ahead is: {time_ahead} tick_count:{tick_count}");
        if tick_count >= self.max_tick_per_update {
            self.max_tick_per_update
        } else {
            tick_count
        }
    }

    #[inline]
    pub fn performed_tick_count(&mut self, tick_count: u16) {
        self.consumed_absolute_time +=
            MillisDuration::from_millis(tick_count as u64 * self.tick_time_duration.as_millis())
    }
}

pub type MillisDurationRange = RangeToFactor<MillisDuration, MillisDuration>;

pub struct RangeToFactor<V, F> {
    range_min: V,
    min_factor: F,
    range_max: V,
    max_factor: F,
    factor: F,
}

impl<V: PartialOrd, F> RangeToFactor<V, F> {
    pub const fn new(range_min: V, range_max: V, min_factor: F, factor: F, max_factor: F) -> Self {
        Self {
            range_min,
            min_factor,
            range_max,
            max_factor,
            factor,
        }
    }

    pub fn calculate(&self, input: V) -> &F {
        if input < self.range_min {
            &self.min_factor
        } else if input > self.range_max {
            &self.max_factor
        } else {
            &self.factor
        }
    }
}
