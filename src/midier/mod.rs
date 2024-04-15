mod errors;

use std::{
    error::Error,
    io::{stdin, stdout, Write},
    sync::mpsc::{self, Receiver},
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
) -> Result<midir::MidiInputConnection<T>, errors::MidiError>
where
    F: FnMut(u64, &[u8], &mut T) + Send + 'static,
{
    let Ok(midi_in) = midir::MidiInput::new("midier input") else {
        return Err(errors::MidiError::CantCreateMidiIn);
    };

    println!("Available Midi ports to connect to:");
    let ports = midi_in.ports();
    for (i, port) in ports.iter().enumerate() {
        println!("{i}:   {}", midi_in.port_name(port).unwrap());
    }

    let num = ask("Select one: ");

    let port_in = ports.get(num).unwrap();
    let name = midi_in.port_name(port_in).unwrap();

    let conn_in = midi_in
        .connect(port_in, "midier input port", callback, data)
        .unwrap();

    println!("Connection to {name} is open. Callback will be called on receive.");

    Ok(conn_in)
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
) -> Result<Receiver<(u64, KeyEvent)>, Box<dyn Error>> {
    let (tx, rx) = mpsc::channel();
    let _midi_in = create_midi_input_and_connect(
        move |stamp, msg, tx_local| {
            let midimsg = MidiMessage::from(msg);
            if let MidiMessage::NoteOn(c, k) = midimsg {
                if let Some(channel) = channel {
                    if c != (channel - 1).into() {
                        return;
                    }
                }
                tx_local.send((stamp, k)).unwrap()
            };
        },
        tx.clone(),
    );

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
        );

        assert!(conn.is_ok(), "Connection wasnt correctly created")
    }
}
