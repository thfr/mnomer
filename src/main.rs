mod audiosignal;
mod beatplayer;
mod repl;

use audiosignal::{frequency_relative_semitone_equal_temperament, ToneConfiguration};
use beatplayer::{BeatPattern, BeatPatternType, BeatPlayer};
use repl::repl::{BuiltInOverwriteError, Repl};
use std::convert::TryFrom;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create the tone configurations for the beatplayer
    let freq = 440.0;
    let normal_beat = ToneConfiguration {
        frequency: freq,
        sample_rate: 48000.0, // may be changed by the beatplayer to match the audio device
        length: 0.05,         // 50 ms
        overtones: 1,
        channels: 1,
    };

    // accentuated beat is 5 semitones higher than the normal beat
    let accentuated_beat = ToneConfiguration {
        frequency: frequency_relative_semitone_equal_temperament(freq, 5.0),
        ..normal_beat
    };

    // beatplayer takes care of generating the beat and its playback
    let beatplayer = BeatPlayer::new(
        100,
        4,
        normal_beat,
        accentuated_beat,
        BeatPattern(vec![
            BeatPatternType::Accent,
            BeatPatternType::Beat,
            BeatPatternType::Beat,
            BeatPatternType::Beat,
        ]),
    );

    // create the user interface, the Read Evaluate Print Loop (REPL)
    let mut repl = Repl::new(beatplayer, "♩♩♩♩: ".to_string());

    add_repl_commands(&mut repl)?;

    repl.run()?;

    Ok(())
}

fn add_repl_commands(repl: &mut Repl<BeatPlayer>) -> Result<(), BuiltInOverwriteError> {
    repl.set_command(
        // ENTER to toggle playback
        "".to_string(),
        Box::new(|_, bp: &mut BeatPlayer| {
            let msg = if bp.is_playing() {
                bp.stop();
                String::from("Stop playback")
            } else {
                bp.play_beat()?;
                String::from("Start playback")
            };
            Ok(msg)
        }),
        None,
    )?;

    repl.set_command(
        "start".to_string(),
        Box::new(|_, bp: &mut BeatPlayer| {
            let msg = if !bp.is_playing() {
                bp.play_beat()?;
                "Start playback".to_string()
            } else {
                "Playback already running".to_string()
            };
            Ok(msg)
        }),
        Some("Start the metronome".to_string()),
    )?;

    repl.set_command(
        "stop".to_string(),
        Box::new(|_, bp: &mut BeatPlayer| {
            if !bp.is_playing() {
                Err(String::from("Playback is not running"))
            } else {
                bp.stop();
                Ok(String::from("Playback stopped"))
            }
        }),
        Some("Stop the metronome".to_string()),
    )?;

    repl.set_command(
        "bpm".to_string(),
        Box::new(|args, bp: &mut BeatPlayer| match args {
            Some(bpm_str) => match bpm_str.parse::<u16>() {
                Ok(bpm) => {
                    if !bp.set_bpm(bpm) {
                        return Err(format!("Could not set bpm value of {}", bpm));
                    }
                    Ok(format!("Bpm set to {}", bpm))
                }
                Err(_) => Err(format!("Could not parse \"{}\" to a value", bpm_str)),
            },
            None => Err("No bpm value supplied".to_string()),
        }),
        Some(format!(
            "{}\n  {}",
            "\"bpm <value>\" where <value> >= 1",
            "This value is based on a beat value of 4 (1/4 note value)"
        )),
    )?;

    repl.set_command(
        "pattern".to_string(),
        Box::new(|args, bp: &mut BeatPlayer| match args {
            Some(pattern_str) => {
                let pattern = BeatPattern::try_from(pattern_str.as_str())?;
                bp.set_pattern(&pattern)?;
                Ok(format!("Pattern set to {}", pattern))
            }
            None => Err("No pattern found".to_string()),
        }),
        Some(format!(
            "{}\n  {}\n  {}",
            "\"pattern <pattern>\"",
            "<pattern> must be in the form of `[!|+|.]*`",
            "`!` = accentuated beat  `+` = normal beat  `.` = pause"
        )),
    )?;

    repl.set_command(
        "pitch".to_string(),
        Box::new(|args, bp: &mut BeatPlayer| {
            let pitches: Vec<f64> = match args {
                Some(pitches) => pitches
                    .split(' ')
                    .filter_map(|x| x.parse::<f64>().ok())
                    .collect(),
                None => return Err("No pattern found".to_string()),
            };
            if pitches.len() != 2 {
                return Err("Wrong number of pitches".to_string());
            };
            bp.set_pitches(pitches[0], pitches[1])?;
            Ok(format!("Pitch set to {} and {}", pitches[0], pitches[1]))
        }),
        Some(format!(
            "{}\n  {}",
            "\"pitch <accentuated beat pitch> <normal beat pitch>\"",
            "pitches must should within [20; 20k]Hz"
        )),
    )?;

    repl.set_command(
        "value".to_string(),
        Box::new(|args, bp: &mut BeatPlayer| match args {
            Some(beat_value_str) => match beat_value_str.parse::<u16>() {
                Ok(beat_value) => {
                    if !bp.set_beat_value(beat_value) {
                        return Err(format!("Could not set beat value of {}", beat_value));
                    }
                    Ok(format!("Beat value set to {}", beat_value))
                }
                Err(_) => Err(format!("Could not parse \"{}\" to a value", beat_value_str)),
            },
            None => Err("No beat value supplied".to_string()),
        }),
        Some(format!(
            "{}\n  {}",
            "\"value <note value subdivision for beat pattern>\"",
            "defaults to 4 (beat has a 1/4 note value which is the base for the bpm value)"
        )),
    )?;

    Ok(())
}
