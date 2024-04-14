use std::{error::Error, io::stdout};

use crossterm::{
    event::{self, KeyCode::Char, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, style::Stylize, widgets::Paragraph, Terminal};

use super::TuiArgs;

pub fn tui(_args: TuiArgs) -> Result<(), Box<dyn Error>> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    // main tui loop
    let mut written = String::new();
    loop {
        terminal.draw(|f| {
            let area = f.size();
            f.render_widget(
                Paragraph::new(format!(
                    "Type something: {}\nPress 'q' to quit",
                    written.trim_end()
                ))
                .black()
                .bold()
                .on_light_green(),
                area,
            )
        })?;

        // handle input here
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Char(chr) = key.code {
                        match chr {
                            'q' | 'Q' => break,
                            _ => {
                                written.push(chr);
                            }
                        }
                    } else if key.code == event::KeyCode::Backspace {
                        written.pop();
                    } else if key.code == event::KeyCode::Enter {
                        written.push('\n');
                    }
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
