use monotonic_time_rs::{Millis, MillisDuration};
use time_tick::{RangeToFactor, TimeTick};

#[test]
fn range() {
    let x = RangeToFactor::new(-5, 1, "low", "mid", "high");
    assert_eq!(x.calculate(-20), &"low");
    assert_eq!(x.calculate(-2), &"mid");
    assert_eq!(x.calculate(2), &"high");
}

#[test]
fn time_tick() {
    let mut now = Millis::new(0);

    let mut timer = TimeTick::new(now, MillisDuration::from_millis(10), 100);

    now += MillisDuration::from_millis(20);
    assert_eq!(timer.update(now), 2);
    timer.performed_tick_count(2);
    now += MillisDuration::from_millis(9);
    assert_eq!(timer.update(now), 0);
}

#[test]
fn time_tick_change_duration() {
    let mut now = Millis::new(0);

    let mut timer = TimeTick::new(now, MillisDuration::from_millis(10), 100);

    now += MillisDuration::from_millis(20);
    assert_eq!(timer.update(now), 2);
    timer.performed_tick_count(2);
    now += MillisDuration::from_millis(9);
    timer.set_time_period(MillisDuration::from_millis(9));
    assert_eq!(timer.update(now), 1);
    timer.performed_tick_count(1);

    now += MillisDuration::from_millis(8);
    assert_eq!(timer.update(now), 0);
}
