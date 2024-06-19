use std::{
    collections::VecDeque,
    io,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use rand::Rng;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::*, *},
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    commands::{
        self, ArpeggioArgs, BrokerMode, CCArgs, CombinerArgs, DrumArgs, DrumOutput, MetronomeArgs,
        MidiBrokerArgs, OutputMode, RunArgs,
    },
    messages::{CombinerMessage, CombinerUpdate, MetronomeMessage, MidiTuiMessage, NetworkMessage},
    midier,
    utils::get_last_sent,
};

use super::popup::{PopupError, PopupInput};

const NETWORK_HISTORY_LEN: usize = 50;

macro_rules! check_parse {
    ($self:ident, $result:ident) => {
        if $result.is_err() {
            $self.mode = AppMode::Error(
                "Couldn't parse".into(),
                "Please enter a valid answer".into(),
            );
            return;
        }
    };
    ($self:ident, $result:ident, $message:expr) => {
        if $result.is_err() {
            $self.mode = AppMode::Error("Couldn't parse".into(), $message.into());
            return;
        }
    };
}

macro_rules! ask_question {
    ($self:ident, $question:expr, $next:expr) => {
        $self.question = $question.into();
        $self.input.reset();
        $self.mode = AppMode::Setup($next, false);
    };
    ($self:ident, $question:expr, $next:expr, $options:expr ) => {
        $self.question = $question.into();
        $self.input.reset();
        $self.options = $options.iter().map(|s| s.to_string()).collect();
        $self.mode = AppMode::Setup($next, true);
    };
}

#[derive(Default, Debug)]
pub enum AppMode {
    #[default]
    Normal,
    /// The user is in setup mode, and has answered the given amount of questions
    /// the second value specifies whether the options need to be shown
    Setup(u8, bool),
    /// An error by some child thread: (title, message)
    Error(String, String),
}

#[derive(Debug)]
#[allow(dead_code)] // the fields whose functionalities are not implemented yet
pub struct App {
    mode: AppMode,
    errors: Arc<Mutex<VecDeque<(String, String)>>>,
    network: Vec<(f64, f64)>,
    network_min: f64,
    network_max: f64,
    metronome: f64,
    midi_notes: Vec<u8>,
    midi_args: MidiBrokerArgs,
    network_args: RunArgs,
    combiner_args: CombinerArgs,
    arpeggio_args: ArpeggioArgs,
    cc_args: CCArgs,
    /// The index of the pane with which the user is currently interacting
    active_pane: u8,
    exit: bool,
    input: Input,
    options: Vec<String>,
    question: String,
    midi_tx: Sender<MidiTuiMessage>,
    midi_rx: Receiver<MidiTuiMessage>,
    metronome_tx: Sender<MetronomeMessage>,
    metronome_rx: Receiver<MetronomeMessage>,
    nw_rx: Receiver<NetworkMessage>,
    combiner_rx: Receiver<CombinerMessage>,
    combiner_tx: Sender<CombinerUpdate>,
}

impl Default for App {
    fn default() -> Self {
        let (metronome_tx, metronome_rx) = mpsc::channel();
        let (midi_tx, midi_rx) = mpsc::channel();
        let (_, combiner_rx) = mpsc::channel();
        let (combiner_tx, _) = mpsc::channel();

        Self {
            mode: Default::default(),
            errors: Arc::new(Mutex::new(VecDeque::new())),
            network: Default::default(),
            network_min: Default::default(),
            network_max: Default::default(),
            metronome: Default::default(),
            midi_notes: Default::default(),
            midi_args: Default::default(),
            network_args: Default::default(),
            combiner_args: Default::default(),
            arpeggio_args: Default::default(),
            cc_args: Default::default(),
            active_pane: Default::default(),
            exit: Default::default(),
            input: Default::default(),
            question: Default::default(),
            options: Default::default(),
            midi_rx,
            midi_tx,
            metronome_tx,
            metronome_rx,
            combiner_rx,
            combiner_tx,
            nw_rx: {
                let (_, rx) = mpsc::channel();
                rx
            },
        }
    }
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut super::ui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
            self.handle_messages();
            self.handle_errors();
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // TODO: make sure that getting data from child threads is read often enough
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_messages(&mut self) {
        while let Some(message) = get_last_sent(&self.midi_rx) {
            self.handle_midi_message(message);
        }
        while let Some(message) = get_last_sent(&self.nw_rx) {
            self.handle_network_message(message);
        }
        while let Some(message) = get_last_sent(&self.metronome_rx) {
            self.handle_metronome_message(message);
        }
        while let Some(message) = get_last_sent(&self.combiner_rx) {
            self.handle_combiner_message(message);
        }
    }

    fn handle_errors(&mut self) {
        let Ok(mut errors) = self.errors.try_lock() else {
            return;
        };

        if let Some(error) = errors.pop_front() {
            self.mode = AppMode::Error(format!("Component {} errored", error.0), error.1);
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.mode {
            AppMode::Normal => match key_event.code {
                KeyCode::Char('Q') => self.exit(),
                KeyCode::Char('1') => self.active_pane = 0,
                KeyCode::Char('2') => self.active_pane = 1,
                KeyCode::Char('3') => self.active_pane = 2,
                KeyCode::Char('4') => self.active_pane = 3,
                KeyCode::Char('h') => {
                    self.active_pane =
                        (((self.active_pane % 2) - 1) % 2) + (self.active_pane / 2) * 2
                }
                KeyCode::Char('l') => {
                    self.active_pane =
                        (((self.active_pane % 2) + 1) % 2) + (self.active_pane / 2) * 2
                }
                KeyCode::Char('j') => {
                    self.active_pane = (self.active_pane + 2) % 4;
                }
                KeyCode::Char('k') => {
                    self.active_pane = (self.active_pane - 2) % 4;
                }
                KeyCode::Char('a') => {
                    let mut rng = rand::thread_rng();
                    let value: f64 = rng.gen_range(0.0..1.0);
                    self.handle_network_message(NetworkMessage::Output(value));
                }
                KeyCode::Char('s') => {
                    self.enter_setup();
                }
                KeyCode::Tab => {
                    self.active_pane = (self.active_pane + 1) % 4;
                }
                _ => {
                    if self.active_pane == 3 {
                        match key_event.code {
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                // increase the threshold with 0.05
                                self.combiner_args.threshold += 0.05;
                                let _ = self.combiner_tx.send(CombinerUpdate::Threshold(0.05));
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                // decrease the threshold with 0.05
                                self.combiner_args.threshold -= 0.05;
                                let _ = self.combiner_tx.send(CombinerUpdate::Threshold(-0.05));
                            }
                            _ => {}
                        }
                    }
                }
            },
            AppMode::Setup(_, _) => match key_event.code {
                KeyCode::Enter => {
                    self.handle_setup_input();
                    self.input.reset();
                }
                KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                _ => {
                    self.input.handle_event(&Event::Key(key_event));
                }
            },
            AppMode::Error(_, _) => match key_event.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                _ => {}
            },
        }
    }

    fn enter_setup(&mut self) {
        match self.active_pane {
            0 => {
                // drop old connection
                (_, self.midi_rx) = mpsc::channel();
                ask_question!(self, "Enter the midi channel to listen on [1-16]", 0);
            }
            1 => {
                // drop old connection and setup new one
                (_, self.metronome_rx) = mpsc::channel();
                ask_question!(self, "Press enter to start the metronome device", 0);
            }
            2 => {
                ask_question!(self, "Enter the model's name", 0);
            }
            3 => {
                ask_question!(
                    self,
                    "Select an output mode",
                    0,
                    ["Drum MIDI", "Drum Robot", "Arpeggio", "CC"]
                );
            }
            _ => {
                todo!()
            }
        }
    }

    fn handle_setup_input(&mut self) {
        match self.active_pane {
            // MIDI
            0 => self.handle_midi_setup(),
            // METRONOME
            1 => {
                // start the metronome
                self.start_metronome();
                self.mode = AppMode::Normal;
            }
            2 => self.handle_network_setup(),
            // COMBINER
            3 => self.handle_combiner_setup(),
            _ => {}
        }
    }

    fn handle_midi_setup(&mut self) {
        let AppMode::Setup(step, _) = self.mode else {
            return;
        };

        match step {
            0 => {
                let channel = self.input.value().parse();
                check_parse!(self, channel, "Enter a valid channel [1-16]");
                self.midi_args.channel = Some(channel.unwrap());
                ask_question!(self, "Enter the midi mode to listen on [Single/chord]", 1);
            }
            1 => {
                // handle last question and quit setup mode
                let mode = self.input.value();
                match mode {
                    "chord" => {
                        self.midi_args.mode = BrokerMode::Chord;
                        ask_question!(self, "Enter the chord size", 2);
                    }
                    _ => {
                        self.midi_args.mode = BrokerMode::Single;
                        let options = midier::available_devices(true);
                        ask_question!(self, "Select the input device", 3, options);
                    }
                };
            }
            2 => {
                let chord_size = self.input.value().parse();
                check_parse!(self, chord_size, "Enter a valid (integer) chord size");
                self.midi_args.chord_size = chord_size.unwrap();
                let options = midier::available_devices(true);
                ask_question!(self, "Enter the input device name", 3, options);
            }
            3 => {
                let device = self.input.value().parse();
                check_parse!(self, device, "Enter a valid device number");
                self.midi_args.device = Some(device.unwrap());
                self.midi_args.output_publish = None;
                self.start_midi();
                self.mode = AppMode::Normal;
            }
            _ => {
                self.mode = AppMode::Normal;
            }
        }
    }

    fn handle_network_setup(&mut self) {
        match self.mode {
            AppMode::Setup(0, _) => {
                let model = self.input.value();
                self.network_args.model = model.into();
                ask_question!(self, "Enter the timestep [ms]", 1);
            }
            AppMode::Setup(1, _) => {
                let timestep = self.input.value().parse();
                check_parse!(self, timestep, "Enter a valid (float) timestep [ms]");
                self.network_args.timestep = timestep.unwrap();
                self.start_network();
                self.mode = AppMode::Normal;
            }
            _ => {
                self.mode = AppMode::Normal;
            }
        }
    }

    fn handle_combiner_setup(&mut self) {
        match self.mode {
            AppMode::Setup(0, _) => {
                let mode: Result<usize, _> = self.input.value().parse();
                check_parse!(self, mode, "Please enter a valid option");
                self.input.reset();
                self.combiner_args.output = match mode.unwrap() {
                    0 => {
                        // DRUM MIDI
                        ask_question!(
                            self,
                            "Select an output device",
                            1,
                            midier::available_devices(false)
                        );
                        OutputMode::Drum(DrumArgs::default())
                    }
                    1 => {
                        // DRUM ROBOTIC
                        let drumargs = DrumArgs {
                            output: DrumOutput::Robot,
                            ..Default::default()
                        };
                        ask_question!(self, "Enter the model's shift [ms]", 20);
                        OutputMode::Drum(drumargs)
                    }
                    2 => {
                        // ARPEGGIO
                        let options = midier::available_devices(false);
                        ask_question!(self, "Select output midi device", 5, options);
                        OutputMode::Arpeggio(ArpeggioArgs::default())
                    }
                    3 => {
                        // CC
                        let options = midier::available_devices(false);
                        ask_question!(self, "Select output midi device", 16, options);
                        OutputMode::CC(CCArgs::default())
                    }
                    _ => {
                        self.mode = AppMode::Error(
                            "Invalid option".into(),
                            "Please enter a valid option".into(),
                        );
                        return;
                    }
                };
            }
            AppMode::Setup(1, _) => {
                let device = self.input.value().parse();
                check_parse!(self, device, "Please enter a valid device number");
                self.combiner_args.device = Some(device.unwrap());
                ask_question!(self, "Select an output note", 2);
            }
            AppMode::Setup(2, _) => {
                let note: Result<u8, _> = self.input.value().parse();
                check_parse!(self, note, "Please enter a valid note");
                self.combiner_args.note = note.unwrap();
                ask_question!(self, "Enter a threshold value", 3);
            }
            AppMode::Setup(3, _) => {
                let threshold: Result<f32, _> = self.input.value().parse();
                check_parse!(self, threshold, "Please enter a valid threshold [float]");
                self.combiner_args.threshold = threshold.unwrap();
                ask_question!(self, "How much to subdivide the metronome beat?", 4);
            }
            AppMode::Setup(4, _) => {
                let subdivision = self.input.value().parse();
                check_parse!(
                    self,
                    subdivision,
                    "Please enter a valid subdivision [integer]"
                );
                self.combiner_args.subdivision = subdivision.unwrap();
                self.question = "".into();
                self.input.reset();
                self.start_combiner();
                self.mode = AppMode::Normal;
            }
            AppMode::Setup(20, _) => {
                let shift = self.input.value().parse();
                check_parse!(self, shift, "Enter a valid shift [ms]");
                if let OutputMode::Drum(ref mut drumargs) = self.combiner_args.output {
                    drumargs.shift = shift.unwrap();
                }
                ask_question!(self, "Enter the robot's delay [ms]", 21);
            }
            AppMode::Setup(21, _) => {
                let delay = self.input.value().parse();
                check_parse!(self, delay, "Enter a valid delay [ms]");
                if let OutputMode::Drum(ref mut drumargs) = self.combiner_args.output {
                    drumargs.delay = delay.unwrap();
                }
                ask_question!(self, "Enter a threshold value [float]", 3);
            }
            AppMode::Setup(5, _) => {
                // ARPEGGIO, channel question
                let device = self.input.value().parse();
                check_parse!(self, device, "Please enter a valid device number");
                self.combiner_args.device = Some(device.unwrap());

                ask_question!(self, "What midi channel to output to?", 6);
            }
            AppMode::Setup(6, _) => {
                // ARPEGGIO, channel question
                let channel = self.input.value().parse();
                check_parse!(self, channel, "Please enter a valid channel [1-16]");
                self.arpeggio_args.channel = channel.unwrap();

                ask_question!(self, "Give a subdivision amount", 7);
            }
            AppMode::Setup(7, _) => {
                let subdivision = self.input.value().parse();
                check_parse!(
                    self,
                    subdivision,
                    "Please enter a valid subdivision [integer]"
                );
                self.combiner_args.subdivision = subdivision.unwrap();

                self.combiner_args.output = OutputMode::Arpeggio(self.arpeggio_args.clone());
                self.question = "".into();
                self.input.reset();
                self.start_combiner();
                self.mode = AppMode::Normal;
            }
            AppMode::Setup(16, _) => {
                // ARPEGGIO, channel question
                let device = self.input.value().parse();
                check_parse!(self, device, "Please enter a valid device number");
                self.combiner_args.device = Some(device.unwrap());

                ask_question!(self, "What midi channel to output to?", 17);
            }
            AppMode::Setup(17, _) => {
                // CC, channel question
                let channel = self.input.value().parse();
                check_parse!(self, channel, "Please enter a valid channel [1-16]");
                self.cc_args.channel = channel.unwrap();

                ask_question!(self, "Which CC value should be adjusted", 18);
            }
            AppMode::Setup(18, _) => {
                let cc = self.input.value().parse();
                check_parse!(self, cc, "Please enter a valid CC value [integer 0-127]");
                self.cc_args.cc = cc.unwrap();

                self.question = "".into();
                self.input.reset();
                self.combiner_args.output = OutputMode::CC(self.cc_args.clone());
                self.start_combiner();
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn start_midi(&mut self) {
        let args = self.midi_args.clone();
        let (tx, rx) = mpsc::channel();
        self.midi_rx = rx;
        let errors = Arc::clone(&self.errors);
        thread::spawn(move || {
            let _ = commands::broke(args, Some(tx)).map_err(|e| {
                errors
                    .lock()
                    .unwrap()
                    .push_back(("midi".to_string(), e.to_string()))
            });
        });
    }

    fn start_metronome(&mut self) {
        let args = MetronomeArgs::default();
        let (tx, rx) = mpsc::channel();
        self.metronome_rx = rx;
        let errors = Arc::clone(&self.errors);
        thread::spawn(move || {
            let _ = commands::metronome(args, Some(tx)).map_err(|e| {
                errors
                    .lock()
                    .unwrap()
                    .push_back(("metronome".to_string(), e.to_string()))
            });
        });
    }

    fn start_network(&mut self) {
        let args = self.network_args.clone();
        let (tx, rx) = mpsc::channel();
        self.nw_rx = rx;
        let errors = Arc::clone(&self.errors);
        thread::spawn(move || {
            let _ = commands::run(args, Some(tx)).map_err(|e| {
                errors
                    .lock()
                    .unwrap()
                    .push_back(("network".to_string(), e.to_string()))
            });
        });
    }

    fn start_combiner(&mut self) {
        let args = self.combiner_args.clone();
        let (comb_tx, comb_rx) = mpsc::channel();
        self.combiner_rx = comb_rx;
        let (update_tx, update_rx) = mpsc::channel();
        self.combiner_tx = update_tx;
        let errors = Arc::clone(&self.errors);
        thread::spawn(move || {
            let _ = commands::combine(args, Some(comb_tx), Some(update_rx)).map_err(|e| {
                errors
                    .lock()
                    .unwrap()
                    .push_back(("combiner".to_string(), e.to_string()))
            });
        });
    }

    fn handle_network_message(&mut self, message: NetworkMessage) {
        match message {
            NetworkMessage::Output(value) => {
                let last_time = self.network.last().map(|(time, _)| *time).unwrap_or(0.0);
                self.network.push((last_time + 1.0, value));
                if value < self.network_min {
                    self.network_min = value;
                }
                if value > self.network_max {
                    self.network_max = value;
                }
                if self.network.len() > NETWORK_HISTORY_LEN {
                    self.network.remove(0);
                }
            }
            NetworkMessage::Error(e) => {
                self.mode = AppMode::Error("Reservoir error".into(), e);
            }
        }
    }

    fn handle_metronome_message(&mut self, message: MetronomeMessage) {
        match message {
            MetronomeMessage::Tempo(t) => {
                self.metronome = t;
            }
            MetronomeMessage::MidiOptions(o) => {
                self.options = o;
            }
            MetronomeMessage::MidiSelected(_) => {}
            MetronomeMessage::Error(e) => {
                self.mode = AppMode::Error("Metronome error".into(), e);
            }
        }
    }

    fn handle_midi_message(&mut self, message: MidiTuiMessage) {
        match message {
            MidiTuiMessage::MidiNotes(notes) => {
                self.midi_notes = notes;
            }
            MidiTuiMessage::Error(e) => {
                self.mode = AppMode::Error("Midi error".into(), e);
            }
            MidiTuiMessage::Heartbeat => {}
            MidiTuiMessage::MidiOptions(o) => {
                self.options = o;
            }
            MidiTuiMessage::MidiSelected(_) => {}
        }
    }

    fn handle_combiner_message(&mut self, message: CombinerMessage) {
        match message {
            CombinerMessage::Heartbeat => {}
            CombinerMessage::Output(_) => {}
        }
    }

    fn get_midi_text(&self) -> Text {
        let lines = vec![
            Line::from(vec![
                "listening on: ".into(),
                self.midi_args.channel_str().yellow(),
            ]),
            Line::from(vec!["mode: ".into(), self.midi_args.mode_str().yellow()]),
            Line::from(vec![
                "midi notes: ".into(),
                format!("{:?}", self.midi_notes).yellow(),
            ]),
        ];

        Text::from(lines)
    }

    fn get_metronome_text(&self) -> Text {
        let bpm = self.metronome * 60.0;
        let lines = vec![Line::from(vec![
            "bpm: ".into(),
            format!("{:.1}", bpm).yellow(),
        ])];

        Text::from(lines)
    }

    fn get_network_text(&self) -> Text {
        Text::from(Line::from(vec![
            "last output: ".into(),
            format!(
                "{:.3}",
                self.network.last().map(|(_, value)| *value).unwrap_or(0.0)
            )
            .yellow(),
        ]))
    }

    fn get_combine_text(&self) -> Text {
        let mut lines = vec![];
        lines.push(Line::from(vec![
            "output mode: ".into(),
            self.combiner_args.output.to_string().yellow(),
        ]));

        if matches!(
            self.combiner_args.output,
            OutputMode::Drum(_) | OutputMode::Arpeggio(_)
        ) {
            lines.push(Line::from(vec![
                "subdivision: ".into(),
                self.combiner_args.subdivision.to_string().yellow(),
            ]));
            lines.push(Line::from(vec![
                "threshold: ".into(),
                format!("{:.2}", self.combiner_args.threshold).yellow(),
            ]));
            if let OutputMode::Drum(drumargs) = self.combiner_args.output {
                if matches!(drumargs.output, DrumOutput::Robot) {
                    lines.push(Line::from(vec![
                        "compensation: ".into(),
                        format!("{:.2}", drumargs.delay).yellow(),
                        " ms".into(),
                    ]));
                }
            }
        }

        Text::from(lines)
    }

    fn get_network_dataset(&self) -> Dataset {
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::LightGreen))
            .data(&self.network)
    }

    fn get_network_chart(&self) -> Chart {
        let first_time = self.network.first().map(|(time, _)| *time).unwrap_or(0.0);
        let last_time = self.network.last().map(|(time, _)| *time).unwrap_or(0.0);

        // Create the X axis and define its properties
        let x_axis = Axis::default().bounds([first_time, last_time]);

        // Create the Y axis and define its properties
        let y_axis = Axis::default().bounds([self.network_min, self.network_max]);

        Chart::new(vec![self.get_network_dataset()])
            .x_axis(x_axis)
            .y_axis(y_axis)
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

fn create_block<'a>(
    title: &'a str,
    active: bool,
    instructions: &[(&'a str, &'a str)],
) -> Block<'a> {
    let border_style = if active {
        Style::default().fg(Color::LightGreen)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut block = Block::default()
        .title(
            Title::from(title.bold())
                .alignment(Alignment::Center)
                .position(Position::Top),
        )
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(border_style);

    if active && !instructions.is_empty() {
        let mut insns = vec![];
        instructions.iter().for_each(|(action, key)| {
            insns.push(format!(" {}: ", *action).into());
            insns.push(format!("<{}> ", *key).bold());
        });
        let instructions = Title::from(Line::from(insns));
        block = block.title(
            instructions
                .alignment(Alignment::Center)
                .position(Position::Bottom),
        );
    }

    block
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(1)])
            .split(area);

        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(33), Constraint::Fill(1)])
            .split(main_layout[0]);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Fill(1)])
            .split(outer_layout[0]);

        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(70), Constraint::Fill(1)])
            .split(outer_layout[1]);

        let instructions = Title::from(Line::from(vec![
            " Quit: ".into(),
            "<Q> ".bold(),
            "| Active Pane: ".into(),
            "<tab> [1-4] [hjkl] ".bold(),
        ]));

        let status_line = Block::default()
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::LightGreen));

        let midi_block = create_block(" midi (1) ", self.active_pane == 0, &[]);
        midi_block.render(top[0], buf);
        let midi_content = Layout::vertical([Constraint::Fill(1)])
            .margin(1)
            .split(top[0]);

        let metro_block = create_block(" metronome (2) ", self.active_pane == 1, &[]);
        metro_block.render(top[1], buf);
        let metro_content = Layout::vertical([Constraint::Fill(1)])
            .margin(1)
            .split(top[1]);

        let nw_block = create_block(" reservoir (3) ", self.active_pane == 2, &[]);
        nw_block.render(bottom[0], buf);
        let nw_content = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
            .margin(1)
            .split(bottom[0]);

        let combine_block = create_block(
            " combiner (4) ",
            self.active_pane == 3,
            &[("thresh", " +/- ")],
        );
        combine_block.render(bottom[1], buf);
        let combine_content = Layout::vertical([Constraint::Fill(1)])
            .margin(1)
            .split(bottom[1]);

        status_line.clone().render(main_layout[1], buf);

        Paragraph::new(self.get_midi_text())
            .block(Block::default())
            .render(midi_content[0], buf);

        Paragraph::new(self.get_metronome_text())
            .block(Block::default())
            .render(metro_content[0], buf);

        Paragraph::new(self.get_network_text())
            .block(Block::default())
            .render(nw_content[0], buf);

        self.get_network_chart()
            .block(Block::default())
            .render(nw_content[1], buf);

        Paragraph::new(self.get_combine_text())
            .block(Block::default())
            .render(combine_content[0], buf);

        // render the popup if needed

        match &self.mode {
            AppMode::Setup(_, false) => {
                let popup_area = centered_rect(50, 20, 60, 7, area);
                let popup = PopupInput {
                    title: "Setup",
                    question: &self.question,
                    options: None,
                    input: self.input.value(),
                };

                popup.render(popup_area, buf);
            }
            AppMode::Setup(_, true) => {
                let popup_area = centered_rect(50, 20, 60, 10, area);
                let popup = PopupInput {
                    title: "Setup",
                    question: &self.question,
                    options: Some(&self.options),
                    input: self.input.value(),
                };

                popup.render(popup_area, buf);
            }
            AppMode::Error(t, e) => {
                let popup_area = centered_rect(50, 20, 60, 4, area);
                let popup = PopupError { title: t, error: e };
                popup.render(popup_area, buf);
            }
            AppMode::Normal => {}
        }
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, min_x: u16, min_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Min(min_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Min(min_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
