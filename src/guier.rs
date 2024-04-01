/*!
* Simple module for showing some information in the terminal
*/

use std::fmt::Display;

pub struct Row {
    description: String,
    value: String,
}

impl Row {
    pub fn new<T>(description: &str, value: T) -> Row
    where
        T: Display,
    {
        Row {
            description: description.to_string(),
            value: format!("{}", value),
        }
    }

    pub fn update<T>(&mut self, value: &T)
    where
        T: Display,
    {
        self.value = format!("{}", value);
    }
}

pub struct Gui {
    name: String,
    rows: Vec<Row>,
}

impl Gui {
    pub fn new(name: &str) -> Gui {
        Gui {
            name: name.to_string(),
            rows: Vec::new(),
        }
    }

    pub fn add_row<T>(&mut self, description: &str, value: T)
    where
        T: Display,
    {
        self.rows.push(Row::new(description, value));
    }

    pub fn update_row<T>(&mut self, description: &str, value: &T)
    where
        T: Display,
    {
        for row in &mut self.rows {
            if row.description == description {
                row.update(value);
            }
        }
    }

    pub fn show(&self) {
        print!("\x1B[2J");
        println!();
        println!("\x1b[1;92m{}\x1b[0m", self.name);
        println!();
        for row in &self.rows {
            println!("{}: {}", row.description, row.value);
        }
    }
}
