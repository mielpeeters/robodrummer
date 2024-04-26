use derive_setters::Setters;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::*, *},
};

#[derive(Setters, Debug, Default)]
pub struct PopupInput<'a> {
    #[setters(into)]
    pub title: &'a str,
    #[setters(into)]
    pub question: &'a str,
    pub input: &'a str,
}

#[derive(Setters, Debug, Default)]
pub struct PopupError<'a> {
    #[setters(into)]
    pub title: &'a str,
    #[setters(into)]
    pub error: &'a str,
}

impl Widget for PopupInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // clear everything underneath the popup area
        Clear.render(area, buf);

        let block = Block::new()
            .title(
                Title::from(format!(" {} ", self.title).bold())
                    .alignment(Alignment::Center)
                    .position(Position::Top),
            )
            .borders(Borders::ALL)
            .border_set(border::ROUNDED);

        block.render(area, buf);

        let popup_chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)])
            .margin(1)
            .split(area);

        let question = Text::styled(self.question, Style::default().fg(Color::LightGreen).bold());
        let input = Text::from(format!("> {}", self.input));

        Paragraph::new(question).render(popup_chunks[0], buf);
        Paragraph::new(input).render(popup_chunks[1], buf);
    }
}

impl Widget for PopupError<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // clear everything underneath the popup area
        Clear.render(area, buf);

        let block = Block::new()
            .title(
                Title::from(format!("  {} ", self.title))
                    .alignment(Alignment::Center)
                    .position(Position::Top),
            )
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::new().red().bold());

        block.render(area, buf);

        let popup_chunks = Layout::vertical([Constraint::Fill(1)])
            .margin(1)
            .split(area);

        let error_message = Text::styled(self.error, Style::default().fg(Color::LightRed).bold());

        Paragraph::new(error_message).render(popup_chunks[0], buf);
    }
}
