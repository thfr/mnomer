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
    {
        println!("Sine {}Hz, {}s:", freq, length);
        for sample in &sine.signal {
            print!("{} ", sample);
        }
        println!();
    }

    sine.highpass_20hz();
    sine.lowpass_20khz();
    {
        println!("Highpass filter with 20Hz and lowpass filter with 20kHz");
        for sample in &sine.signal {
            print!("{} ", sample);
        }
        println!();
    }

    let fade_time = 0.00025;
    match sine.fade_in_out(fade_time, fade_time) {
        Ok(_) => {
            println!("Fade in {}s, fade out {}s:", fade_time, fade_time);
            for sample in &sine.signal {
                print!("{} ", sample);
            }
        }
        _ => {
            println!("fade_in_out() did not work");
        }
    }
    println!();

    let beatplayer = BeatPlayer {
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
