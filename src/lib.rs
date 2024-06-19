/*!
* This crate contains all code for the RoboDrummer project.
*
* This is a binary crate, meaning that it is intended to be compiled into a standalone executable.
* Although one binary is created, it has multiple modes of operation, depending on which subcommand you provide.
* These binaries then communicate with each other using ZMQ sockets, which means that there is no
* one order in which they need to be started.
*
* The different subcommands are:
* - [`generate-data`]: Generate training data based on rule based methods like Euclidean rhythms.
* - [`train`]: Train a reservoir computer on the generated data.
* - [`run`]: Run the trained reservoir.
* - [`completions`]: Generate shell completions for the CLI. (zsh only)
* - [`midi-broker`]: Runs the MIDI broker, which lies between the MIDI device and the rest of the
* system.
* - [`combine`]: Combine the MIDI input, the model output, and the metronome output to create the
* desired output.
* - [`metronome`] : Currently not part of this crate, but a separate one.
*
* [`midi-broker`]: commands/fn.broke.html
* [`combine`]: commands/fn.combine.html
* [`completions`]: commands/fn.update_completions.html
* [`generate-data`]: commands/fn.gendata.html
* [`run`]: commands/fn.run.html
* [`train`]: commands/fn.train.html
* [`metronome`]: commands/fn.metronome.html
*
* # Compilation
* Clone this repository, then run:
* ```sh
* cargo build --release
* ```
* This will create the binary at `./target/release/robodrummer`.
* You can also install it to be available in your PATH:
* ```sh
* cargo install --path .
* ```
*
* # Usage
* The basic usage would be the following:
* ```sh
* robodrummer generate-data -o my_data euclidean -n 8 -k 3
* ```
* This generates a dataset of Euclidean rhythms with 8 pulses and 3 onsets.
* <br>
* To train a reservoir computer on this data, you can run:
* ```sh
* robodrummer train --data my_data
* ```
* This will train a random reservoir of 300 neurons, and plot the data if Python with `matplotlib` and `pandas` is installed.
* The binary will prompt you to give the trained model a name.
* <br>
* To run the trained model, you can run:
* ```sh
* robodrummer tui
* ```
* This will thart the Terminal User Interface, which will allow you to setup the just-trained model,
* along with the other components (midi-broker, metronome, and output component).
*/

pub mod activation;
pub mod arpeggio;
pub mod commands;
pub mod constants;
pub mod data;
pub mod errors;
pub mod guier;
pub mod hyper;
pub mod messages;
pub mod metronomer;
pub mod midier;
pub mod midiutils;
pub mod oscutil;
pub mod reservoir;
pub mod robot;
pub mod series;
pub mod test_robot;
pub mod trainutil;
pub mod tui;
pub mod utils;
