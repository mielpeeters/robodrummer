use std::f32::consts::PI;

pub fn sine_with(period: i32, size: f32, offset: f32, phase: f32) -> Box<dyn Fn(i32) -> f32> {
    Box::new(move |i| (i as f32 * PI * 2.0 / period as f32 + phase).sin() * size + offset)
}

pub fn saw_with(period: i32, size: f32, offset: f32, phase: f32) -> Box<dyn Fn(i32) -> f32> {
    Box::new(move |i| ((i as f32 / period as f32 + phase) % 1.0) * size - size / 2.0 + offset)
}

pub fn constant(value: f32) -> Box<dyn Fn(i32) -> f32> {
    Box::new(move |_| value)
}

pub fn linear(period: i32, start: f32, end: f32) -> Box<dyn Fn(i32) -> f32> {
    Box::new(move |i| {
        let t = i as f32 / period as f32;
        (1.0 - t) * start + t * end
    })
}
