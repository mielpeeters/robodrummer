/*!
* Simple module for showing some information in the terminal
*/

use std::fmt::Display;

const GRAPH_WIDTH: usize = 70;
const GRAPH_HEIGHT: usize = 20;

pub enum Row {
    Text(Text),
    Graph(Graph),
}

pub struct Text {
    description: String,
    value: String,
}

pub struct Graph {
    description: String,
    min: f64,
    max: f64,
    history: Vec<f64>,
}

impl Row {
    pub fn update<T>(&mut self, value: &T)
    where
        T: Display,
    {
        match self {
            Row::Text(text) => text.update(value),
            Row::Graph(g) => g.update(value),
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Row::Text(text) => &text.description,
            Row::Graph(g) => &g.description,
        }
    }

    pub fn show(&self) {
        match self {
            Row::Text(text) => text.show(),
            Row::Graph(g) => g.show(),
        }
    }
}

impl Text {
    pub fn new<T>(description: &str, value: T) -> Self
    where
        T: Display,
    {
        Self {
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

    pub fn show(&self) {
        println!("{}: {}", self.description, self.value);
    }
}

impl Graph {
    pub fn new(description: &str, min: f64, max: f64) -> Self {
        Self {
            description: description.to_string(),
            min,
            max,
            history: Vec::new(),
        }
    }

    pub fn update<T>(&mut self, value: &T)
    where
        T: Display,
    {
        let value = format!("{}", value).parse::<f64>().unwrap();
        self.history.push(value);
        if self.history.len() > GRAPH_WIDTH {
            self.history.remove(0);
        }

        if value < self.min {
            self.min = value;
        }

        if value > self.max {
            self.max = value;
        }
    }

    pub fn replace(&mut self, values: &[f32]) {
        // self.min = (*values
        //     .iter()
        //     .min_by(|a, b| a.partial_cmp(b).unwrap())
        //     .unwrap())
        // .into();
        // self.max = (*values
        //     .iter()
        //     .max_by(|a, b| a.partial_cmp(b).unwrap())
        //     .unwrap())
        // .into();
        self.max = 2.0;
        self.min = -0.5;
        self.history = values.iter().map(|v| *v as f64).collect();
    }

    pub fn show(&self) {
        println!("{}:", self.description);
        let step_size = (self.max - self.min) / GRAPH_HEIGHT as f64;
        println!("{:.2}", self.max);
        for i in (0..GRAPH_HEIGHT).rev() {
            let y = self.min + (self.max - self.min) * i as f64 / GRAPH_HEIGHT as f64;
            let mut line = String::new();
            for x in 0..self.history.len() {
                let value = self.history[x];
                if value < y && value > y - step_size {
                    let chr = if x == (self.history.len() - 1) {
                        '*'
                    } else {
                        '_'
                    };
                    line.push(chr);
                } else {
                    line.push(' ');
                }
            }
            println!("{}", line);
        }
        println!("{:.2}", self.min);
    }
}

pub struct Gui {
    name: String,
    rows: Vec<Row>,
    enabled: bool,
}

impl Gui {
    pub fn new(name: &str) -> Gui {
        Gui {
            name: name.to_string(),
            rows: Vec::new(),
            enabled: true,
        }
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn add_row<T>(&mut self, description: &str, value: T)
    where
        T: Display,
    {
        self.rows.push(Row::Text(Text::new(description, value)));
    }

    pub fn add_graph(&mut self, description: &str, min: f64, max: f64) {
        self.rows
            .push(Row::Graph(Graph::new(description, min, max)));
    }

    pub fn update_row<T>(&mut self, description: &str, value: &T)
    where
        T: Display,
    {
        for row in &mut self.rows {
            if row.description() == description {
                row.update(value);
            }
        }
    }

    pub fn replace_graph(&mut self, description: &str, values: &[f32]) {
        for row in &mut self.rows {
            if row.description() == description {
                if let Row::Graph(g) = row {
                    g.replace(values);
                }
            }
        }
    }

    pub fn show(&self) {
        if !self.enabled {
            return;
        }
        print!("\x1B[2J");
        println!();
        println!("\x1b[1;92m{}\x1b[0m", self.name);
        println!();
        for row in &self.rows {
            row.show();
        }
    }
}
