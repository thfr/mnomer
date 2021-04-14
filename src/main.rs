mod audiosignal;
mod beatplayer;
mod repl;

use audiosignal::{freqency_relative_semitone_equal_temperament, ToneConfiguration};
use beatplayer::{BeatPattern, BeatPatternType, BeatPlayer};
use repl::Repl;
use std::convert::TryFrom;
use std::sync::Mutex;

fn main() {
    // Create the tone configurations for the beatplayer
    let freq = 440.0;
    let length = 0.05;
    let overtones = 1;
    let normal_beat = ToneConfiguration {
        frequency: freq,
        sample_rate: 48000.0,
        length,
        overtones,
        channels: 1,
    };
    // accentuated beat is 5 semitones higher than the normal beat
    let accentuated_beat = ToneConfiguration {
        frequency: freqency_relative_semitone_equal_temperament(freq, 5.0),
        ..normal_beat
    };

    // beatplayer takes care of generating the beat and its playback
    let beatplayer = Mutex::new(BeatPlayer::new(
        100,
        normal_beat.clone(),
        accentuated_beat.clone(),
        BeatPattern {
            0: vec![
                BeatPatternType::Accent,
                BeatPatternType::Beat,
                BeatPatternType::Beat,
                BeatPatternType::Beat,
            ],
        },
    ));

    // define user commands: keyord + closure
    let commands: Vec<(String, Box<dyn FnMut(Option<&str>, &mut BeatPlayer)>)> = vec![
        (
            // ENTER to toggle playback
            "".to_string(),
            Box::new(|_, bp: &mut BeatPlayer| {
                if bp.is_playing() {
                    bp.stop();
                } else {
                    match bp.play_beat() {
                        Ok(_) => (),
                        Err(y) => println!("{}", y),
                    };
                }
                println!("{}", bp.to_string());
                println!("");
            }),
        ),
        (
            "start".to_string(),
            Box::new(|_, bp: &mut BeatPlayer| {
                if !bp.is_playing() {
                    match bp.play_beat() {
                        Ok(_) => (),
                        Err(y) => println!("{}", y),
                    };
                }
                println!("{}", bp.to_string());
                println!("");
            }),
        ),
        (
            "stop".to_string(),
            Box::new(|_, bp: &mut BeatPlayer| {
                bp.stop();
                println!("{}", bp.to_string());
                println!("");
            }),
        ),
        (
            "bpm".to_string(),
            Box::new(|args, bp: &mut BeatPlayer| {
                let print_help = || println!("Command usage: bpm <value> \n  where <value> >= 1");
                match args {
                    Some(bpm_str) => match bpm_str.parse::<u16>() {
                        Ok(bpm) => {
                            if !bp.set_bpm(bpm) {
                                println!("Could not set bpm value of {}", bpm);
                            }
                        }
                        Err(_) => {
                            println!("Could not parse \"{}\" to a value", bpm_str);
                            print_help();
                        }
                    },
                    None => {
                        println!("No bpm value supplied");
                        print_help();
                    }
                }
                println!("{}", bp.to_string());
                println!("");
            }),
        ),
        (
            "pattern".to_string(),
            Box::new(|args, bp: &mut BeatPlayer| {
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
                            Ok(_) => (),
                        },
                        Err(x) => {
                            println!("{}", x);
                            print_help();
                        }
                    },
                    None => {
                        println!("No pattern found");
                        print_help();
                    }
                }
                println!("{}", bp.to_string());
                println!("");
            }),
        ),
    ];

    // create and start the user interface, the Read Evaluate Print Loop (REPL)
    let mut repl = Repl {
        app: beatplayer,
        commands,
        exit: false.into(),
        prompt: "♩♩♩♩: ".to_string(),
    };
    repl.process();
}
