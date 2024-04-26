/*!
* Connecting to the robot using the audio card's output
*/

use std::{
    collections::VecDeque,
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{sleep, JoinHandle},
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait},
    Stream, SupportedStreamConfig,
};

/// Wave types should be able to generate a vector of samples
#[derive(Debug, Clone, Copy)]
pub enum WaveType {
    /// A pulse wave with given width in seconds
    Pulse(f32),
    /// A sine wave with given frequency and width
    Sine(f32, f32),
    /// A single sawtooth with given width
    Saw(f32),
    /// A parabolic sloping descent
    Slope(f32),
    /// A saw that slowly goes up too
    SlowSaw(f32),
}

impl WaveType {
    pub fn generate(&self, sample_rate: f32) -> VecDeque<f32> {
        let mut res = VecDeque::new();
        match self {
            WaveType::Pulse(w) => {
                let width = (sample_rate * w) as usize;
                for _ in 0..width {
                    res.push_back(1.0);
                }
            }
            WaveType::Sine(f, w) => {
                let width = (sample_rate * w) as usize;
                for i in 0..width {
                    res.push_back((2.0 * PI * f * i as f32 / sample_rate).sin());
                }
            }
            WaveType::Saw(w) => {
                let width = (sample_rate * w) as usize;
                for i in 0..width {
                    res.push_back(1.0 - (i as f32 / width as f32))
                }
            }
            WaveType::SlowSaw(w) => {
                let width = (sample_rate * w) as usize;
                let fifth = width / 5;
                for i in 0..fifth {
                    res.push_back(i as f32 / fifth as f32)
                }
                for i in 0..width - fifth {
                    res.push_back(1.0 - (i as f32 / (width - fifth) as f32))
                }
            }
            WaveType::Slope(w) => {
                let width = (sample_rate * w) as usize;
                for i in 0..width {
                    res.push_back(1.0 - (i as f32 / width as f32).powf(3.0))
                }
            }
        }

        res
    }
}

// pub fn addBeat()

/// Start the audio engine and pass signals to the robot
///
/// # Arguments
/// - `send_beat` : A boolean flag which indicates a beat should be sent
pub fn start(
    send_beat: Arc<AtomicBool>,
    wave: WaveType,
) -> (Stream, SupportedStreamConfig, JoinHandle<()>) {
    let host = cpal::host_from_id(
        cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect("features = ['jack'] should be added to the Cargo.toml file"),
    )
    .expect("jack host should be available");

    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();

    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;

    log::info!("Config of the output: {:#?}", config.config());

    let err_fn = |err| eprintln!("an error occurred on input stream: {err}");

    // queue keeps track of the beats' samples that need to be sent
    let queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));

    let output_queue = Arc::clone(&queue);

    let out_stream = device.build_output_stream(
        &config.config(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let mut sample = output_queue.lock().unwrap();
                let sample = sample.pop_front().unwrap_or(0.0);
                for ch in frame {
                    *ch = sample;
                }
            }
        },
        err_fn,
        None,
    );

    // start the thread that receives the beat signal
    let handle = std::thread::spawn(move || loop {
        if send_beat.load(Ordering::Relaxed) {
            let samples = wave.generate(sample_rate);
            let mut queue = queue.lock().unwrap();
            *queue = samples;
            send_beat.store(false, Ordering::Relaxed);
        }

        // check approx every millisecond
        // NOTE: could be a source of additional delay
        sleep(Duration::from_millis(1));
    });

    (out_stream.unwrap(), config, handle)
}
