/*!
  This module implements some useful timeseries to use as input / output data
  for training the network.
*/

use std::f64::consts::PI;

/// this macro improves the readability of the definition of all of the
/// different series
macro_rules! serie {
    ($name:ident ($($arg_name: ident: $arg_type: ty),*) $fun: expr) => {
        pub fn $name($($arg_name: $arg_type),*) -> Box<dyn Fn(i32) -> f64> {
            #![allow(unused)]
            Box::new($fun)
        }
    };
}

serie!(sine_with(period: i32, size: f64, offset: f64, phase: f64) move |i| {
    (i as f64 * PI * 2.0 / period as f64 + phase).sin() * size + offset
});

serie!(saw_with(period: i32, size: f64, offset: f64, phase: f64) move |i| {
    ((i as f64 / period as f64 + phase) % 1.0) * size - size / 2.0 + offset
});

serie!(constant(value: f64) move |_| value);

serie!(linear(period: i32, start: f64, end: f64) move |i| {
    let t = i as f64 / period as f64;
    (1.0 - t) * start + t * end
});

serie!(spike(period: i32, height: f64, decay: f64) move |i| {
    height * (- (i % period) as f64 / decay).exp()
});

serie!(sine_speed_up(period: i32, size: f64, amount: f64, time: i32) move |i| {
    let speedup = (1.0 + amount * (i as f64 / time as f64));
    (i as f64 * PI * 2.0 * speedup / period as f64).sin() * size
});

serie!(impulse_pause(size: f64) move |i| {
    if i == 0 {
        size
    } else {
        0.0
    }
});

serie!(impulse_width_pause(size: f64, width: i32) move |i| {
    if i < width {
        size
    } else {
        0.0
    }
});
