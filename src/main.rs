mod audiosignal;
mod beatplayer;
mod repl;

use audiosignal::{freqency_relative_semitone_equal_temperament, ToneConfiguration};
use beatplayer::{BeatPattern, BeatPatternType, BeatPlayer};
use repl::{CommandDefinition, InputHistory, Repl};
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
        4,
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
        status_line: "This is mnomer".to_string(),
        history: InputHistory::new(),
    };

    add_repl_commands(&mut repl);

    match repl.run_with_crossterm() {
        Ok(_) => (),
        Err(_) => println!("Something went wrong with the REPL"),
    };
}

fn add_repl_commands(repl: &mut Repl<BeatPlayer>) {
    repl.set_command(CommandDefinition {
        // ENTER to toggle playback
        command: "".to_string(),
        function: Box::new(|_, bp: &mut BeatPlayer| {
            let msg = if bp.is_playing() {
                bp.stop();
                String::from("Stop playback")
            } else {
                bp.play_beat()?;
                String::from("Start playback")
            };
            Ok(msg)
        }),
        help: None,
    });

    repl.set_command(CommandDefinition {
        command: "start".to_string(),
        function: Box::new(|_, bp: &mut BeatPlayer| {
            let msg = if !bp.is_playing() {
                bp.play_beat()?;
                String::from("Start playback")
            } else {
                String::from("Playback already running")
            };
            Ok(msg)
        }),
        help: None,
    });

    repl.set_command(CommandDefinition {
        command: "stop".to_string(),
        function: Box::new(|_, bp: &mut BeatPlayer| {
            if !bp.is_playing() {
                Err(String::from("Playback is not running"))
            } else {
                bp.stop();
                Ok(String::from("Playback stopped"))
            }
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
                        Ok(format!("Bpm set to {}", bpm))
                    }
                    Err(_) => {
                        Err(format!("Could not parse \"{}\" to a value", bpm_str))
                    }
                },
                None => {
                    Err(format!("No bpm value supplied"))
                }
            }
        }),
        help: Some(String::from("\"bpm <value>\" where <value> >= 1\nThis value is based on a beat value of 4 (1/4 note duration)")),
    });

    repl.set_command(CommandDefinition {
        command: "pattern".to_string(),
        function: Box::new(|args, bp: &mut BeatPlayer| match args {
            Some(pattern_str) => {
                let pattern = BeatPattern::try_from(pattern_str.as_str())?;
                bp.set_pattern(&pattern)?;
                return Ok(format!("Pattern set to {}", pattern));
            }
            None => return Err(format!("No pattern found")),
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
                        Err(_) => false,
                    })
                    .map(|x| x.parse::<f64>().unwrap())
                    .collect(),
                None => return Err(format!("No pattern found")),
            };
            if pitches.len() != 2 {
                return Err(String::from("Wrong number of pitches"));
            };
            bp.set_pitches(pitches[0], pitches[1])?;
            Ok(format!("Pitch set to {} and {}", pitches[0], pitches[1]))
        }),
        help: Some(String::from(format!(
            "\"pitch <accentuated beat pitch> <normal beat pitch>\" pitches must should within [20; 20k]Hz"
        ))),
    });
    repl.set_command(CommandDefinition {
        command: "beatvalue".into(),
        function: Box::new(|args, bp: &mut BeatPlayer| match args {
            Some(beat_value_str) => match beat_value_str.parse::<u16>() {
                Ok(beat_value) => {
                    if !bp.set_beat_value(beat_value) {
                        return Err(format!("Could not set beat value of {}", beat_value));
                    }
                    Ok(format!("Beat value set to {}", beat_value))
                }
                Err(_) => Err(format!("Could not parse \"{}\" to a value", beat_value_str)),
            },
            None => Err(format!("No beat value supplied")),
        }),
        help: Some(String::from(format!(
            "{}\n{}",
            "\"beatvalue <note value for beat pattern>\"",
            "  defaults to 4 (meaning a beat is a 1/4 note which is the base for the bpm value)",
        ))),
    });
}
