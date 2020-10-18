mod audiosignal;
mod beatplayer;
mod repl;

use audiosignal::AudioSignal;
use beatplayer::BeatPlayer;
use repl::Repl;
use std::sync::{Arc, Mutex};

fn main() {
    let freq = 440.0;
    let length = 0.05;
    let overtones = 1;
    let mut sine = AudioSignal::generate_tone(freq, length, overtones);

    sine.highpass_20hz();
    sine.lowpass_20khz();

    let fade_time = 0.01;
    sine.fade_in_out(fade_time, fade_time).unwrap();

    let beatplayer = Arc::new(Mutex::new(BeatPlayer::new(
        80,
        sine.clone(),
        sine.clone(),
        vec![true],
    )));

    let bp_empty_string = beatplayer.clone();
    let bp_start = beatplayer.clone();
    let bp_stop = beatplayer.clone();
    let bp_bpm = beatplayer.clone();

    let mut repl = Repl {
        commands: vec![
            (
                "".to_string(),
                Box::new(move |_| {
                    let mut bp = bp_empty_string.lock().unwrap();
                    if bp.is_playing() {
                        bp.stop();
                    } else {
                        match bp.play_beat() {
                            Ok(_) => (),
                            Err(_) => (),
                        };
                    }
                    println!("");
                }),
            ),
            (
                "start".to_string(),
                Box::new(move |_| {
                    let mut bp = bp_start.lock().unwrap();
                    if !bp.is_playing() {
                        match bp.play_beat() {
                            Ok(_) => (),
                            Err(_) => (),
                        };
                    }
                    println!("");
                }),
            ),
            (
                "stop".to_string(),
                Box::new(move |_| {
                    let mut bp = bp_stop.lock().unwrap();
                    if bp.is_playing() {
                        bp.stop();
                    }
                    println!("");
                }),
            ),
            (
                "bpm".to_string(),
                Box::new(move |args| {
                    match args {
                        Some(bpm_str) => match bpm_str.parse::<u16>() {
                            Ok(bpm) => {
                                let mut bp = bp_bpm.lock().unwrap();
                                if !bp.set_bpm(bpm) {
                                    println!("Could not set bpm value of {}", bpm);
                                }
                            }
                            Err(_) => println!("Could not parse \"{}\" to a value", bpm_str),
                        },
                        None => println!("No bpm value supplied"),
                    }
                    println!("");
                }),
            ),
        ],
        exit: false,
        prompt: "♩♩♩♩: ".to_string(),
    };
    repl.process();
}
