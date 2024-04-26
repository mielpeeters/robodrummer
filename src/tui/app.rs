use std::{
    io,
    sync::mpsc::{self, Receiver},
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
    commands::{self, BrokerMode, MidiBrokerArgs, RunArgs},
    utils::get_last_sent,
};

use super::{
    messages::{MidiTuiMessage, NetworkMessage},
    popup::{PopupError, PopupInput},
};

const NETWORK_HISTORY_LEN: usize = 50;

macro_rules! check_parse {
    ($self:ident, $result:ident) => {
        if $result.is_err() {
            $self.mode = AppMode::Error(
                "Couldn't parse".into(),
                "Please enter a valid channel number".into(),
            );
            return;
        }
    };
}

#[derive(Default, Debug)]
pub enum AppMode {
    #[default]
    Normal,
    /// The user is in setup mode, and has answered the given amount of questions
    Setup(u8),
    /// An error by some child thread: (title, message)
    Error(String, String),
}

#[derive(Debug)]
pub struct App {
    mode: AppMode,
    network: Vec<(f64, f64)>,
    network_min: f64,
    network_max: f64,
    metronome: f32,
    midi_notes: Vec<u8>,
    midi: MidiBrokerArgs,
    network_args: RunArgs,
    /// The index of the pane with which the user is currently interacting
    active_pane: u8,
    exit: bool,
    input: Input,
    question: String,
    midi_rx: Receiver<MidiTuiMessage>,
    nw_rx: Receiver<NetworkMessage>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            network: Default::default(),
            network_min: Default::default(),
            network_max: Default::default(),
            metronome: Default::default(),
            midi_notes: Default::default(),
            midi: Default::default(),
            network_args: Default::default(),
            active_pane: Default::default(),
            exit: Default::default(),
            input: Default::default(),
            question: Default::default(),
            midi_rx: {
                let (_, rx) = mpsc::channel();
                rx
            },
            nw_rx: {
                let (_, rx) = mpsc::channel();
                rx
            },
        }
    }
}

// impl Default for App {
//     fn default() -> Self {
//         Self {

//         }
//     }
// }

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut super::ui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
            self.handle_messages();
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
                _ => {}
            },
            AppMode::Setup(_) => match key_event.code {
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
                self.question = "Enter the midi channel to listen on".into();
                self.input.reset();
            }
            2 => {
                self.question = "Enter the model's name".into();
                self.input.reset();
            }
            _ => {
                todo!()
            }
        }

        self.mode = AppMode::Setup(0);
    }

    fn handle_setup_input(&mut self) {
        match self.active_pane {
            // MIDI
            0 => {
                if let AppMode::Setup(i) = self.mode {
                    match i {
                        0 => {
                            let channel = self.input.value().parse();
                            self.input.reset();
                            check_parse!(self, channel);
                            self.midi.channel = Some(channel.unwrap());
                            self.question =
                                "Enter the midi mode to listen on [Single/chord]".into();
                            self.mode = AppMode::Setup(1);
                        }
                        1 => {
                            // handle last question and quit setup mode
                            let mode = self.input.value();
                            match mode {
                                "chord" => {
                                    self.midi.mode = BrokerMode::Chord;
                                    self.question = "Enter the chord size".into();
                                    self.input.reset();
                                    self.mode = AppMode::Setup(2);
                                }
                                _ => {
                                    self.midi.mode = BrokerMode::Single;
                                    self.question = "Enter the device name".into();
                                    self.input.reset();
                                    self.mode = AppMode::Setup(3);
                                }
                            };
                        }
                        2 => {
                            let chord_size = self.input.value().parse();
                            check_parse!(self, chord_size);
                            self.midi.chord_size = chord_size.unwrap();
                            self.question = "Enter the device name".into();
                            self.input.reset();
                            self.mode = AppMode::Setup(3);
                        }
                        3 => {
                            self.midi.device = Some(self.input.value().to_string());
                            self.question = "".into();
                            self.input.reset();
                            self.start_midi();
                            self.mode = AppMode::Normal;
                        }
                        _ => {
                            self.mode = AppMode::Normal;
                        }
                    }
                }
            }
            2 => {
                if let AppMode::Setup(i) = self.mode {
                    match i {
                        0 => {
                            let model = self.input.value();
                            self.network_args.model = model.into();
                            self.input.reset();
                            self.question = "Enter the timestep [ms]".into();
                            self.mode = AppMode::Setup(1);
                        }
                        1 => {
                            let timestep = self.input.value();
                            let timestep = timestep.parse();
                            check_parse!(self, timestep);
                            self.network_args.timestep = timestep.unwrap();
                            self.input.reset();
                            self.start_network();
                            self.mode = AppMode::Normal;
                        }
                        _ => {
                            self.mode = AppMode::Normal;
                        }
                    }
                }
            }
            _ => {
                todo!()
            }
        }
    }

    fn start_network(&mut self) {
        let args = self.network_args.clone();
        let (tx, rx) = mpsc::channel();
        self.nw_rx = rx;
        thread::spawn(move || commands::run(args, Some(tx)).unwrap());
    }

    fn start_midi(&mut self) {
        let args = self.midi.clone();
        let (tx, rx) = mpsc::channel();
        self.midi_rx = rx;
        // TODO: Midi message sender
        thread::spawn(move || commands::broke(args, Some(tx)).unwrap());
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
        }
    }

    fn get_midi_text(&self) -> Text {
        let lines = vec![
            Line::from(vec![
                "listening on: ".into(),
                self.midi.channel_str().yellow(),
            ]),
            Line::from(vec!["mode: ".into(), self.midi.mode_str().yellow()]),
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
            self.network
                .last()
                .map(|(_, value)| *value)
                .unwrap_or(0.0)
                .to_string()
                .yellow(),
        ]))
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

    fn get_combine_text(&self) -> Text {
        Text::from("TODO")
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
fn create_block(title: &str, active: bool) -> Block {
    let border_style = if active {
        Style::default().fg(Color::LightGreen)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .title(
            Title::from(title.bold())
                .alignment(Alignment::Center)
                .position(Position::Top),
        )
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(border_style)
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
            .constraints(vec![Constraint::Percentage(40), Constraint::Fill(1)])
            .split(outer_layout[1]);

        let instructions = Title::from(Line::from(vec![
            " Quit: ".into(),
            "<Q> ".bold(),
            "| Active Pane: ".into(),
            "<tab> or [1-4] ".bold(),
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

        let midi_block = create_block(" midi (1) ", self.active_pane == 0);
        midi_block.render(top[0], buf);
        let midi_content = Layout::vertical([Constraint::Fill(1)])
            .margin(1)
            .split(top[0]);

        let metro_block = create_block(" metronome (2) ", self.active_pane == 1);
        metro_block.render(top[1], buf);
        let metro_content = Layout::vertical([Constraint::Fill(1)])
            .margin(1)
            .split(top[1]);

        let nw_block = create_block(" network (3) ", self.active_pane == 2);
        nw_block.render(bottom[0], buf);
        let nw_content = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
            .margin(1)
            .split(bottom[0]);

        let combine_block = create_block(" combiner (4) ", self.active_pane == 3);
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
            AppMode::Setup(_) => {
                let popup_area = centered_rect(50, 20, area);
                let popup = PopupInput {
                    title: "Setup",
                    question: &self.question,
                    input: self.input.value(),
                };

                popup.render(popup_area, buf);
            }
            AppMode::Error(t, e) => {
                let popup_area = centered_rect(50, 20, area);
                let popup = PopupError { title: t, error: e };
                popup.render(popup_area, buf);
            }
            AppMode::Normal => {}
        }
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
