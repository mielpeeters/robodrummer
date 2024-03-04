/*!
  This module implements some useful timeseries to use as input / output data
  for training the network.
*/

use std::f32::consts::PI;

/// this macro improves the readability of the definition of all of the
/// different series
macro_rules! serie {
    ($name:ident ($($arg_name: ident: $arg_type: ty),*) $fun: expr) => {
        pub fn $name($($arg_name: $arg_type),*) -> Box<dyn Fn(i32) -> f32> {
            #![allow(unused)]
            Box::new($fun)
        }
    };
}

serie!(sine_with(period: i32, size: f32, offset: f32, phase: f32) move |i| {
    (i as f32 * PI * 2.0 / period as f32 + phase).sin() * size + offset
});

serie!(saw_with(period: i32, size: f32, offset: f32, phase: f32) move |i| {
    ((i as f32 / period as f32 + phase) % 1.0) * size - size / 2.0 + offset
});

serie!(constant(value: f32) move |_| value);

serie!(linear(period: i32, start: f32, end: f32) move |i| {
    let t = i as f32 / period as f32;
    (1.0 - t) * start + t * end
});

serie!(spike(period: i32, height: f32, decay: f32) move |i| {
    height * (- (i % period) as f32 / decay).exp()
});

serie!(sine_speed_up(period: i32, size: f32, amount: f32, time: i32) move |i| {
    let speedup = (1.0 + amount * (i as f32 / time as f32));
    (i as f32 * PI * 2.0 * speedup / period as f32).sin() * size
});

serie!(impulse_pause(size: f32) move |i| {
    if i == 0 {
        size
    } else {
        0.0
    }
});

serie!(impulse_width_pause(size: f32, width: i32) move |i| {
    if i < width {
        size
    } else {
        0.0
    }
});
