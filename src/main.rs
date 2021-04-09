mod audiosignal;
mod beatplayer;
mod repl;

use audiosignal::{freqency_relative_semitone_equal_temperament, ToneConfiguration};
use beatplayer::{BeatPattern, BeatPatternType, BeatPlayer};
use repl::Repl;
use std::convert::TryFrom;
use std::sync::Mutex;

fn main() {
    // Create the tones for the beatplayer
    let freq = 440.0;
    let length = 0.05;
    let overtones = 1;
    let sine = ToneConfiguration {
        frequency: freq,
        sample_rate: 48000.0,
        length,
        overtones,
        channels: 1,
    };
    let accentuated_sine = ToneConfiguration {
        frequency: freqency_relative_semitone_equal_temperament(freq, 5.0),
        ..sine
    };

    // beatplayer takes care of generating the beat and its playback
    let beatplayer = Mutex::new(BeatPlayer::new(
        100,
        sine.clone(),
        accentuated_sine.clone(),
        BeatPattern {
            0: vec![
                BeatPatternType::Accent,
                BeatPatternType::Beat,
                BeatPatternType::Beat,
                BeatPatternType::Beat,
            ],
        },
    ));

    // create the read print evaluate loop with the commands and their associated functions
    let mut repl = Repl {
        app: beatplayer,
        commands: vec![
            (
                "".to_string(),
                Box::new(move |_, bp: &mut BeatPlayer| {
                    if bp.is_playing() {
                        println!("Stopping playback");
                        bp.stop();
                    } else {
                        println!(
                            "Starting playback with bpm {} and pattern {:?}",
                            bp.bpm, bp.pattern
                        );
                        match bp.play_beat() {
                            Ok(_) => (),
                            Err(y) => println!("{}", y),
                        };
                    }
                    println!("");
                }),
            ),
            (
                "start".to_string(),
                Box::new(move |_, bp: &mut BeatPlayer| {
                    println!(
                        "Starting playback with bpm {} and pattern {:?}",
                        bp.bpm, bp.pattern
                    );
                    if !bp.is_playing() {
                        match bp.play_beat() {
                            Ok(_) => (),
                            Err(y) => println!("{}", y),
                        };
                    }
                    println!("");
                }),
            ),
            (
                "stop".to_string(),
                Box::new(move |_, bp: &mut BeatPlayer| {
                    if bp.is_playing() {
                        println!("Stopping playback");
                        bp.stop();
                    }
                    println!("");
                }),
            ),
            (
                "bpm".to_string(),
                Box::new(move |args, bp: &mut BeatPlayer| {
                    match args {
                        Some(bpm_str) => match bpm_str.parse::<u16>() {
                            Ok(bpm) => {
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
            (
                "pattern".to_string(),
                Box::new(move |args, bp: &mut BeatPlayer| {
                    let print_help = || {
                        println!(
                            "{}\n{}\n{}",
                            "Command usage: pattern <pattern>",
                            "  <pattern> must be in the form of `[!|+|.]*`",
                            "  `!` = accentuated beat  `+` = normal beat  `.` = pause"
                        )
                    };
                    match args {
                        Some(pattern_str) => match BeatPattern::try_from(pattern_str) {
                            Ok(pattern) => match bp.set_pattern(pattern) {
                                Err(x) => println!("{}", x),
                                _ => (),
                            },
                            Err(x) => println!("{}", x),
                        },
                        None => {
                            println!("No pattern found");
                            print_help()
                        }
                    }
                    println!("");
                }),
            ),
        ],
        exit: false.into(),
        prompt: "♩♩♩♩: ".to_string(),
    };
    repl.process();
}
