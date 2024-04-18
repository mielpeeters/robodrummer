use std::{fmt::Display, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use rand::Rng;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::*, *},
};

use crate::commands::MidiBrokerArgs;

const NETWORK_HISTORY_LEN: usize = 50;

#[derive(Debug, Default)]
pub struct App {
    network: Vec<(f64, f64)>,
    network_min: f64,
    network_max: f64,
    metronome: f32,
    output: Vec<f32>,
    midi: MidiBrokerArgs,
    /// The index of the pane with which the user is currently interacting
    active_pane: u8,
    exit: bool,
}

#[derive(Debug)]
pub struct TuiMessage<T>
where
    T: Display,
{
    pub key: String,
    pub value: T,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut super::ui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('1') => self.active_pane = 0,
            KeyCode::Char('2') => self.active_pane = 1,
            KeyCode::Char('3') => self.active_pane = 2,
            KeyCode::Char('4') => self.active_pane = 3,
            KeyCode::Char('a') => {
                let mut rng = rand::thread_rng();
                let value: f32 = rng.gen_range(0.0..1.0);
                self.handle_tui_message(TuiMessage {
                    key: "network".into(),
                    value,
                })
            }
            KeyCode::Tab => {
                self.active_pane = (self.active_pane + 1) % 4;
            }
            _ => {
                todo!()
            }
        }
    }

    fn handle_tui_message<T>(&mut self, message: TuiMessage<T>)
    where
        T: Display,
    {
        match message.key.as_str() {
            "metronome" => self.metronome = message.value.to_string().parse().unwrap(),
            "network" => {
                let last_time = self.network.last().map(|(time, _)| *time).unwrap_or(0.0);
                let value = message.value.to_string().parse().unwrap();
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
            _ => {}
        }
    }

    fn get_midi_text(&self) -> Text {
        let lines = vec![
            Line::from(vec![
                "listening on: ".into(),
                self.midi.channel_str().yellow(),
            ]),
            Line::from(vec!["mode: ".into(), self.midi.mode_str().yellow()]),
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
            "<q>".bold(),
            "   Active Pane: ".into(),
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
    }
}
