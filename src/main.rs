extern crate alsa;

// use alsa::*;

fn generate_sine(freq: u32) -> Vec<i16> {
    return vec![0];
}

struct BeatPlayer {
    beat: Vec<i16>,
    accentuated_beat: Vec<i16>,
    pattern: Vec<bool>,
}

struct Repl {
    commands: Vec<(String, fn(&str))>,
}

fn main() {
    let sine = generate_sine(500);
}
