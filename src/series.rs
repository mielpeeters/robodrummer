use std::{f32::consts::PI, process};

macro_rules! serie {
    ($name:ident ($($arg_name: ident: $arg_type: ty),*) $fun: expr) => {
        pub fn $name($($arg_name: $arg_type),*) -> Box<dyn Fn(i32) -> f32> {
            #![allow(unused)]
            Box::new($fun)
        }
    };
}

#[macro_export]
macro_rules! add_data {
    ($target:ident <- [$($shape: ident),*]; $amount: expr) => {
        add_series_data(
            &mut $target,
            &[$($shape.as_ref()),*],
            0..$amount,
        );
    };
}

#[macro_export]
macro_rules! csv_entry {
    ($writer: ident <- $($header:expr),*) => {
        $writer.write_record(&[$(format!("{}", $header).as_str()),*]).expect("ow");
    };
}

#[macro_export]
macro_rules! csv_stop {
    ($writer: ident) => {
        $writer.flush().unwrap();
        drop($writer);
    };
}

#[macro_export]
macro_rules! csv_start {
    ($filename: expr) => {
        csv::Writer::from_path($filename).unwrap()
    };
}

#[macro_export]
macro_rules! python {
    ($filename: expr) => {
        std::process::Command::new("python3")
            .arg($filename)
            .output()
            .unwrap();
    };
}

#[macro_export]
macro_rules! miel {
    ($say: ident; > $($rest: tt)*) => { $say.push_str("right "); miel!($say; $($rest)*); };
    ($say: ident; >> $($rest: tt)*) => { miel!($say; > > $($rest)*); };
    ($say: ident; < $($rest: tt)*) => { $say.push_str("left "); miel!($say; $($rest)*); };
    ($say: ident; << $($rest: tt)*) => { miel!($say; < < $($rest)*); };
    ($say: ident; : $($rest: tt)*) => { $say.push_str("nothn "); miel!($say; $($rest)*); };
    ($say: ident; :: $($rest: tt)*) => { miel!($say; : : $($rest)*); };
    ($say: ident; ) => {};
}

pub fn say(what_to_say: &str) {
    process::Command::new("spd-say")
        .arg("-y")
        .arg("male5")
        .arg(what_to_say)
        .output()
        .unwrap();
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
