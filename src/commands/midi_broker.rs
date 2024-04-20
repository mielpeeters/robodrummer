use std::sync::mpsc::{self, Sender};

use crate::{
    midier,
    tui::{app::TuiMessage, messages::MidiTuiMessage},
};

use super::MidiBrokerArgs;
use midi_control::{KeyEvent, MidiNote};

const FILTER_SIZE: usize = 8;

struct MidiFilter<const L: usize> {
    /// keeps the last L midi messages
    notes: [KeyEvent; L],
    /// keeps track of the timing of the messages (in microseconds)
    stamps: [u64; L],
    /// points to the last-added element
    pointer: usize,
    /// the amount of notes that make up a chord
    chord_size: usize,
    /// the maximum time between notes to be considered a chord
    max_time: u64,
    /// the last note of the last chord that was detected
    last_chord_time: u64,
}

impl<const L: usize> MidiFilter<L> {
    fn new(chord_size: usize, max_time: u64) -> Self {
        assert!(chord_size <= L);
        MidiFilter {
            notes: std::array::from_fn(|_| KeyEvent { key: 0, value: 0 }),
            stamps: [0; L],
            pointer: 0,
            chord_size,
            max_time,
            last_chord_time: 0,
        }
    }

    fn add(&mut self, stamp: u64, note: KeyEvent) {
        self.pointer = (self.pointer + 1) % L;
        self.notes[self.pointer] = note;
        self.stamps[self.pointer] = stamp;
    }

    fn idx_n_back(&self, n: usize) -> usize {
        (self.pointer as i32 - n as i32).rem_euclid(L as i32) as usize
    }

    #[allow(unused)]
    fn show_stamps(&self) {
        for i in 0..L {
            let val = self.stamps[i] as f64 / 1000.0;
            if i == self.pointer {
                log::debug!("{}: {} ms <-", i, val);
            } else {
                log::debug!("{}: {} ms", i, val);
            }
        }
    }

    fn chord_played(&mut self) -> Option<Vec<MidiNote>> {
        let last_time = self.stamps[self.pointer];
        let first_time = self.stamps[self.idx_n_back(self.chord_size - 1)];

        // either the notes are too far apart, or they have been played already in a sent chord
        if last_time - first_time > self.max_time || first_time <= self.last_chord_time {
            None
        } else {
            self.last_chord_time = last_time;
            let mut chord = Vec::new();
            for i in 0..self.chord_size {
                chord.push(self.notes[self.idx_n_back(i)].key);
            }
            Some(chord)
        }
    }
}

fn single(
    rx: mpsc::Receiver<(u64, KeyEvent)>,
    publisher: zmq::Socket,
    tui_sender: Option<Sender<TuiMessage<MidiTuiMessage>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut midi_filter = MidiFilter::<1>::new(1, 100_000);

    loop {
        // wait for incomming midi messages
        let (timestamp, keyevent) = rx.recv()?;

        midi_filter.add(timestamp, keyevent);

        if let Some(chord) = midi_filter.chord_played() {
            publisher.send(chord.clone(), 0)?;
            // update the TUI
            if let Some(sender) = &tui_sender {
                if sender
                    .send(MidiTuiMessage::MidiNotes(chord).into())
                    .is_err()
                {
                    // connection is not live anymore, close this thread
                    break;
                }
            }
        }
    }

    Ok(())
}

fn chord(
    rx: mpsc::Receiver<(u64, KeyEvent)>,
    publisher: zmq::Socket,
    chord_size: usize,
    tui_sender: Option<Sender<TuiMessage<MidiTuiMessage>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(chord_size < FILTER_SIZE);
    let mut midi_filter = MidiFilter::<FILTER_SIZE>::new(chord_size, 100_000);

    loop {
        // wait for incomming midi messages
        let (timestamp, keyevent) = rx.recv()?;

        midi_filter.add(timestamp, keyevent);

        if let Some(chord) = midi_filter.chord_played() {
            log::debug!("send a chord: {:?}", chord);
            publisher.send(chord.clone(), 0)?;
            if let Some(sender) = &tui_sender {
                if sender
                    .send(MidiTuiMessage::MidiNotes(chord).into())
                    .is_err()
                {
                    // connection is not live anymore, close this thread
                    break;
                }
            }
        }
    }

    Ok(())
}

/// handle midi incomping messages
pub fn broke(
    args: MidiBrokerArgs,
    tui_sender: Option<Sender<TuiMessage<MidiTuiMessage>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // set up zmq pubish channel
    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB)?;
    publisher.bind(&format!("tcp://*:{}", args.port))?;

    // set up midi input connection
    let rx = midier::setup_midi_receiver(args.channel)?;

    match args.mode {
        super::BrokerMode::Single => single(rx, publisher, tui_sender),
        super::BrokerMode::Chord => chord(rx, publisher, args.chord_size, tui_sender),
    }
}
