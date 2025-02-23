mod errors;

use std::{
    error::Error,
    io::{stdin, stdout, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc, Condvar, Mutex,
    },
    thread,
};

use midi_control::{KeyEvent, MidiMessage};

fn ask(question: &str) -> usize {
    print!("\n{question}");
    stdout().flush().unwrap();

    let mut answer = String::new();

    stdin().read_line(&mut answer).unwrap();

    answer.trim().parse::<usize>().unwrap()
}

pub fn create_midi_output(name: &str) -> Result<midir::MidiOutput, errors::MidiError> {
    let Ok(output) = midir::MidiOutput::new(name) else {
        return Err(errors::MidiError::CantCreateMidiOut);
    };

    Ok(output)
}

pub fn connect_output(
    output: midir::MidiOutput,
) -> Result<midir::MidiOutputConnection, errors::MidiError> {
    let ports = output.ports();

    let num = {
        println!("Available Midi ports to connect to:");
        for (i, port) in ports.iter().enumerate() {
            println!("\t{i}: {}", output.port_name(port).unwrap());
        }
        let res = ask("Select one: ");
        if res >= ports.len() {
            return Err(errors::MidiError::PortNotOpen);
        }
        res
    };

    let name = output.port_name(ports.get(num).unwrap()).unwrap();

    let Ok(conn) = output.connect(ports.get(num).unwrap(), "midier output port") else {
        return Err(errors::MidiError::CantConnectMidi);
    };

    println!("Connected to Midi device \x1b[1m{}\x1b[0m!", name);

    Ok(conn)
}

pub fn create_midi_output_and_connect(
    device: Option<u8>,
) -> Result<midir::MidiOutputConnection, errors::MidiError> {
    let Ok(output) = midir::MidiOutput::new("midier output") else {
        return Err(errors::MidiError::CantCreateMidiOut);
    };

    let ports = output.ports();

    let num = if let Some(device) = &device {
        *device as usize
    } else {
        println!("Available Midi ports to connect to:");
        for (i, port) in ports.iter().enumerate() {
            println!("\t{i}: {}", output.port_name(port).unwrap());
        }
        let res = ask("Select one: ");
        if res >= ports.len() {
            return Err(errors::MidiError::PortNotOpen);
        }
        res
    };

    let name = output.port_name(ports.get(num).unwrap()).unwrap();

    let Ok(conn) = output.connect(ports.get(num).unwrap(), "midier output port") else {
        return Err(errors::MidiError::CantConnectMidi);
    };

    if device.is_none() {
        println!("Connected to Midi device \x1b[1m{}\x1b[0m!", name);
    }

    Ok(conn)
}

pub fn available_devices(input: bool) -> Vec<String> {
    if input {
        let midi = match midir::MidiInput::new("midier input") {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let mut devices = Vec::new();
        for port in midi.ports() {
            devices.push(midi.port_name(&port).unwrap());
        }

        devices
    } else {
        let midi = match midir::MidiOutput::new("midier output") {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let mut devices = Vec::new();
        for port in midi.ports() {
            devices.push(midi.port_name(&port).unwrap());
        }

        devices
    }
}

pub fn create_midi_input_and_connect<F, T: Send>(
    callback: F,
    data: T,
    device: Option<u8>,
) -> Result<midir::MidiInputConnection<T>, errors::MidiError>
where
    F: FnMut(u64, &[u8], &mut T) + Send + 'static,
{
    let Ok(midi_in) = midir::MidiInput::new("midier input") else {
        return Err(errors::MidiError::CantCreateMidiIn);
    };

    let ports = midi_in.ports();

    let num = if let Some(device) = device {
        device as usize
    } else {
        println!("Available Midi ports to connect to:");
        for (i, port) in ports.iter().enumerate() {
            println!("\t{i}: {}", midi_in.port_name(port).unwrap());
        }
        let res = ask("Select one: ");
        if res >= ports.len() {
            return Err(errors::MidiError::PortNotOpen);
        }
        res
    };

    let port_in = ports.get(num).unwrap();

    let conn_in = midi_in
        .connect(port_in, "midier input port", callback, data)
        .unwrap();

    Ok(conn_in)
}

#[allow(unused)]
fn check_midi_device(available: &str, device: &str) -> bool {
    available.to_lowercase().contains(&device.to_lowercase())
}

// pub fn rcv_signal(conn: &mut midir::MidiInputConnection<>)

pub fn send_beat(conn: &mut midir::MidiOutputConnection, channel: u32) {
    let ch = match channel {
        1 => midi_control::Channel::Ch1,
        2 => midi_control::Channel::Ch2,
        _ => midi_control::Channel::Ch1,
    };

    let msg: Vec<u8> = midi_control::note_on(ch, 60, 100).into();
    conn.send(msg.as_slice()).unwrap();
}

pub fn send_note(conn: &mut midir::MidiOutputConnection, channel: u32, note: u8, velocity: u8) {
    let ch = midi_control::Channel::from(channel as u8);
    let msg: Vec<u8> = midi_control::note_on(ch, note, velocity).into();
    conn.send(msg.as_slice()).unwrap();
}

type MidiNote = (u64, u8, KeyEvent);
pub fn setup_midi_receiver(
    channel: Option<u8>,
    key: Option<u8>,
    device: Option<u8>,
) -> Result<Receiver<MidiNote>, Box<dyn Error>> {
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();
    let (error_tx, error_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let no_connection_left = Arc::new((Mutex::new(false), Condvar::new()));
        let no_connection_left_clone = no_connection_left.clone();
        let midi_in = create_midi_input_and_connect(
            move |stamp, msg, tx_local| {
                let midimsg = MidiMessage::from(msg);
                if let MidiMessage::NoteOn(c, k) = midimsg {
                    // filter on midi channel if needed
                    if let Some(channel) = channel {
                        if c != (channel - 1).into() {
                            return;
                        }
                    }

                    let c = c as u8;

                    // check if the key matches
                    if let Some(desired_key) = key {
                        if k.key != desired_key {
                            return;
                        }
                    }

                    // check if velocity > 0
                    if k.value == 0 {
                        return;
                    }

                    // send the midi keyevent to the main thread
                    if tx_local.send((stamp, c, k)).is_err() {
                        // there is no connection to the main thread anymore
                        *no_connection_left_clone.0.lock().unwrap() = true;
                        no_connection_left_clone.1.notify_all()
                    };
                };
            },
            tx,
            device,
        );

        if let Err(e) = midi_in {
            error_tx.send(e).unwrap();
            return;
        }

        done_clone.store(true, Ordering::Relaxed);

        let mut no_left = no_connection_left.0.lock().unwrap();
        while !*no_left {
            no_left = no_connection_left.1.wait(no_left).unwrap();
        }

        drop(midi_in);
    });

    while !done.load(std::sync::atomic::Ordering::Relaxed) {
        // maybe an error occurred
        if let Ok(e) = error_rx.try_recv() {
            return Err(Box::new(e));
        }

        thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output() {
        let conn = create_midi_output_and_connect(None);

        assert!(conn.is_ok(), "Connection wasnt correctly created")
    }

    #[test]
    fn test_input() {
        let conn = create_midi_input_and_connect(
            move |stamp, msg, _| println!("{} -> {:?} (lenth {})", stamp, msg, msg.len()),
            (),
            None,
        );

        assert!(conn.is_ok(), "Connection wasnt correctly created")
    }
}
