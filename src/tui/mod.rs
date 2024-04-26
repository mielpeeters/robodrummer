use crate::commands::TuiArgs;

use std::error::Error;

pub mod app;
pub mod messages;
pub mod popup;
pub mod ui;

use app::App;

pub fn start_tui(_args: TuiArgs) -> Result<(), Box<dyn Error>> {
    let mut terminal = ui::init()?;
    let app_result = App::default().run(&mut terminal);
    ui::restore()?;
    Ok(app_result?)
}
