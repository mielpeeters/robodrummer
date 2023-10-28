use std::time::Duration;

use neuroner::midier;

fn main() {
    let mut conn = midier::create_midi_output_and_connect().unwrap();
    for _ in 0..1000 {
        midier::send_beat(&mut conn, 1);
        std::thread::sleep(Duration::from_millis(300));
    }
}
