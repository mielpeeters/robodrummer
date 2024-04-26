mod errors;

use std::{
    error::Error,
    io::{stdin, stdout, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
};

use midi_control::{KeyEvent, MidiMessage, MidiMessageSend};

fn ask(question: &str) -> usize {
    print!("\n{question}");
    stdout().flush().unwrap();

    let mut answer = String::new();

    stdin().read_line(&mut answer).unwrap();

    answer.trim().parse::<usize>().unwrap()
}

pub fn create_midi_output_and_connect() -> Result<midir::MidiOutputConnection, errors::MidiError> {
    let Ok(output) = midir::MidiOutput::new("midier output") else {
        return Err(errors::MidiError::CantCreateMidiOut);
    };

    println!("Available Midi ports to connect to:");
    let ports = output.ports();
    for (i, port) in ports.iter().enumerate() {
        println!("{i}:   {}", output.port_name(port).unwrap());
    }

    let num = ask("Select one: ");

    let name = output.port_name(ports.get(num).unwrap()).unwrap();

    let Ok(conn) = output.connect(ports.get(num).unwrap(), "midier output port") else {
        return Err(errors::MidiError::CantConnectMidi);
    };

    println!("Connected to Midi device \x1b[1m{}\x1b[0m!", name);

    Ok(conn)
}

pub fn create_midi_input_and_connect<F, T: Send>(
    callback: F,
    data: T,
    device: Option<String>,
) -> Result<midir::MidiInputConnection<T>, errors::MidiError>
where
    F: FnMut(u64, &[u8], &mut T) + Send + 'static,
{
    let Ok(midi_in) = midir::MidiInput::new("midier input") else {
        return Err(errors::MidiError::CantCreateMidiIn);
    };

    let ports = midi_in.ports();

    let num = if let Some(device) = device {
        ports
            .iter()
            .position(|port| check_midi_device(&midi_in.port_name(port).unwrap(), &device))
            .ok_or(errors::MidiError::DeviceNotFound(device))?
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

    conn.send_message(midi_control::note_on(ch, 60, 100))
        .unwrap();
}

pub fn send_note(conn: &mut midir::MidiOutputConnection, channel: u32, note: u8, velocity: u8) {
    let ch = match channel {
        1 => midi_control::Channel::Ch1,
        2 => midi_control::Channel::Ch2,
        _ => midi_control::Channel::Ch1,
    };

    conn.send_message(midi_control::note_on(ch, note, velocity))
        .unwrap();
}

pub fn setup_midi_receiver(
    channel: Option<u8>,
    device: Option<String>,
) -> Result<Receiver<(u64, KeyEvent)>, Box<dyn Error>> {
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();
    let (error_tx, error_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let no_connection_left = Arc::new(AtomicBool::new(false));
        let no_connection_left_clone = no_connection_left.clone();
        let midi_in = create_midi_input_and_connect(
            move |stamp, msg, tx_local| {
                let midimsg = MidiMessage::from(msg);
                if let MidiMessage::NoteOn(c, k) = midimsg {
                    if let Some(channel) = channel {
                        if c != (channel - 1).into() {
                            return;
                        }
                    }
                    if tx_local.send((stamp, k)).is_err() {
                        no_connection_left_clone.store(true, Ordering::Relaxed)
                    };
                };
            },
            tx.clone(),
            device,
        );

        if let Err(e) = midi_in {
            error_tx.send(e).unwrap();
            return;
        }

        done_clone.store(true, Ordering::Relaxed);

        loop {
            if no_connection_left.load(Ordering::Relaxed) {
                // not needed but this shows the purpose
                drop(midi_in);
                break;
            }
            // keep the thread (thus the midi connection) alive
            thread::sleep(std::time::Duration::from_millis(100000));
        }
    });

    while !done.load(std::sync::atomic::Ordering::Relaxed) {
        // maybe an error occurred
        if let Ok(e) = error_rx.try_recv() {
            return Err(Box::new(e));
        }

        thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output() {
        let conn = create_midi_output_and_connect();

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
