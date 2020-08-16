mod audiosignal;
mod beatplayer;

use audiosignal::AudioSignal;
use beatplayer::BeatPlayer;

// TODO Repl
// struct Repl {
//     commands: Vec<(String, fn(&String))>,
// }
// impl Repl {}

fn main() {
    let freq = 500.0;
    let length = 0.002;
    let mut sine = AudioSignal::generate_tone(freq, length, 3);

    sine.highpass_20hz();
    sine.lowpass_20khz();

    let fade_time = 0.00025;
    sine.fade_in_out(fade_time, fade_time).unwrap();

    let mut beatplayer = BeatPlayer {
        bpm: 80,
        beat: AudioSignal::generate_tone(440.0, 0.05, 1),
        accentuated_beat: sine.clone(),
        playback_buffer: sine.clone(),
        pattern: vec![true],
    };
    match beatplayer.play_beat() {
        Result::Ok(_) => println!("Everything is fine"),
        Result::Err(_) => println!("Error happened"),
    };
}
