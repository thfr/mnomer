mod audiosignal;
mod beatplayer;
mod repl;

use audiosignal::{freqency_relative_semitone_equal_temperament, ToneConfiguration};
use beatplayer::{BeatPattern, BeatPatternType, BeatPlayer};
use repl::{CommandDefinition, Repl};
use std::sync::Mutex;
use std::{collections::HashMap, convert::TryFrom};

fn main() {
    // Create the tone configurations for the beatplayer
    let freq = 440.0;
    let normal_beat = ToneConfiguration {
        frequency: freq,
        sample_rate: 48000.0, // may be changed by the beatplayer for match the audio device
        length: 0.05,         // 50 ms
        overtones: 1,
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
        normal_beat,
        accentuated_beat,
        BeatPattern {
            0: vec![
                BeatPatternType::Accent,
                BeatPatternType::Beat,
                BeatPatternType::Beat,
                BeatPatternType::Beat,
            ],
        },
    ));

    // create the user interface, the Read Evaluate Print Loop (REPL)
    let mut repl = Repl {
        app: beatplayer,
        commands: HashMap::new(),
        exit: false.into(),
        prompt: "♩♩♩♩: ".to_string(),
    };

    add_repl_commands(&mut repl);

    repl.run();
}

fn add_repl_commands(repl: &mut Repl<BeatPlayer>) {
    repl.set_command(CommandDefinition {
        // ENTER to toggle playback
        command: "".to_string(),
        function: Box::new(|_, bp: &mut BeatPlayer| {
            if bp.is_playing() {
                bp.stop();
            } else {
                bp.play_beat()?;
            }
            println!("{}", bp.to_string());
            println!("");
            Ok(())
        }),
        help: None,
    });

    repl.set_command(CommandDefinition {
        command: "start".to_string(),
        function: Box::new(|_, bp: &mut BeatPlayer| {
            if !bp.is_playing() {
                bp.play_beat()?;
            }
            println!("{}", bp.to_string());
            println!("");
            Ok(())
        }),
        help: None,
    });

    repl.set_command(CommandDefinition {
        command: "stop".to_string(),
        function: Box::new(|_, bp: &mut BeatPlayer| {
            bp.stop();
            println!("{}", bp.to_string());
            println!("");
            Ok(())
        }),
        help: None,
    });

    repl.set_command(CommandDefinition {
        command: "bpm".to_string(),
        function: Box::new(|args, bp: &mut BeatPlayer| {
            match args {
                Some(bpm_str) => match bpm_str.parse::<u16>() {
                    Ok(bpm) => {
                        if !bp.set_bpm(bpm) {
                            return Err(format!("Could not set bpm value of {}", bpm));
                        }
                    }
                    Err(_) => {
                        return Err(format!("Could not parse \"{}\" to a value", bpm_str));
                    }
                },
                None => {
                    return Err(format!("No bpm value supplied"));
                }
            }
            println!("{}", bp.to_string());
            println!("");
            Ok(())
        }),
        help: Some(String::from("\"bpm <value>\" where <value> >= 1")),
    });

    repl.set_command(CommandDefinition {
        command: "pattern".to_string(),
        function: Box::new(|args, bp: &mut BeatPlayer| {
            match args {
                Some(pattern_str) => {
                    let pattern = BeatPattern::try_from(pattern_str.as_str())?;
                    bp.set_pattern(pattern)?;
                }
                None => return Err(format!("No pattern found")),
            }
            println!("{}", bp.to_string());
            println!("");
            Ok(())
        }),
        help: Some(String::from(format!(
            "{}\n{}\n{}",
            "\"pattern <pattern>\"",
            "  <pattern> must be in the form of `[!|+|.]*`",
            "  `!` = accentuated beat  `+` = normal beat  `.` = pause"
        ))),
    });

    repl.set_command(CommandDefinition {
        command: "pitch".to_string(),
        function: Box::new(|args, bp: &mut BeatPlayer| {
            let pitches: Vec<f64> = match args {
                Some(pitches) => pitches
                    .split(' ')
                    .filter(|x| match x.parse::<f64>() {
                        Ok(_) => true,
                        Err(_) => {
                            println!("Could not parse {}", x);
                            false
                        }
                    })
                    .map(|x| x.parse::<f64>().unwrap())
                    .collect(),
                None => return Err(format!("No pattern found")),
            };
            if pitches.len() != 2 {
                return Err(String::from("Wrong number of pitches"));
            };
            bp.set_pitches(pitches[0], pitches[1])?;
            println!("{}", bp.to_string());
            println!("");
            Ok(())
        }),
        help: Some(String::from(format!(
            "{}\n{}",
            "\"pitch <accentuated beat pitch> <normal beat pitch>\"",
            "  pitches must should within [20; 20k]Hz, may be floating point numbers",
        ))),
    });
}
