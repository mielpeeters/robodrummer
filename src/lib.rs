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
* - TODO: `metronome`: Currently not part of this crate, but a separate one.
*
* [`midi-broker`]: commands/fn.broke.html
* [`combine`]: commands/fn.combine.html
* [`completions`]: commands/fn.update_completions.html
* [`generate-data`]: commands/fn.gendata.html
* [`run`]: commands/fn.run.html
* [`train`]: commands/fn.train.html
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
* robodrummer run --model my_model
* ```
* This will run the model, and publish the output on a ZMQ socket (default socket is 4321).
*
* <br>
*
* You will then need to run the metronome, which is a separate binary. (see metronomer crate)
*
* <br>
*
* You'll also need to run the MIDI broker:
* ```sh
* robodrummer midi-broker -m single
* ```
* It will ask you to select a MIDI device to get input from.
* This process handles the midi input and publishes it on another ZMQ socket.
*
* <br>
*
* Finally, you can combine the output of the model, the metronome, and the MIDI input:
* ```sh
* robodrummer combine -s 4
* ````
* This will run the combiner, which subdivides the metronome signal, and thresholds the model to
* know which beats will be played and which won't.
*/

mod activation;
mod arpeggio;
pub mod commands;
pub mod constants;
pub mod data;
mod errors;
mod guier;
pub mod metronomer;
pub mod midier;
pub mod midiutils;
pub mod oscutil;
pub mod reservoir;
pub mod robot;
pub mod series;
pub mod trainutil;
mod tui;
mod utils;
