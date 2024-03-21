/// For playing arpeggios out of the combiner
pub struct Arpeggio {
    pub chord: Vec<u8>,
    pub duration: f32,
    pub current: usize,
    /// offset to play the notes at (12 for an octave)
    pub offset: usize,
}

impl Arpeggio {
    pub fn new(chord: &[u8], duration: f32, offset: usize) -> Self {
        let chord = chord.iter().map(|x| x + offset as u8).collect();

        Arpeggio {
            chord,
            duration,
            offset,
            current: 0,
        }
    }

    pub fn next(&mut self) -> u8 {
        self.current = (self.current + 1) % self.chord.len();
        self.chord[self.current]
    }

    pub fn update_chord(&mut self, chord: &[u8]) {
        self.chord = chord.iter().map(|x| x + self.offset as u8).collect();

        log::debug!("Arpeggio: {:?}", self.chord);

        self.current = self.chord.len() - 1;
    }
}
