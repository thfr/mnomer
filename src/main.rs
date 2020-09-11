mod audiosignal;
mod beatplayer;
mod repl;

use audiosignal::AudioSignal;
use beatplayer::BeatPlayer;
use repl::Repl;

fn main() {
    let mut repl = Repl {
        commands: vec![
            ("".to_string(), |_| println!("Found empty string")),
            ("start".to_string(), |_| println!("Found start command")),
            ("bpm".to_string(), |args| {
                print!("Found bpm command");
                match args {
                    Some(bpm) => print!(" with following args: \"{}\"", bpm),
                    None => print!(" with no args"),
                }
                println!("");
            }),
        ],
        exit: false,
        prompt: "♩♩♩♩: ".to_string()
    };
    repl.process();
}

fn test_beatplayer() {
    let freq = 500.0;
    let length = 0.002;
    let overtones = 3;
    let mut sine = AudioSignal::generate_tone(freq, length, overtones);

    sine.highpass_20hz();
    sine.lowpass_20khz();

    let fade_time = 0.00025;
    sine.fade_in_out(fade_time, fade_time).unwrap();

    let mut beatplayer = BeatPlayer::new(
        80,
        AudioSignal::generate_tone(440.0, 0.05, 1),
        sine.clone(),
        vec![true],
    );

    match beatplayer.play_beat() {
        Result::Ok(_) => println!("Everything is fine, playing beat"),
        Result::Err(_) => println!("Error happened"),
    };

    println!("Sleeping for 5s");
    std::thread::sleep(std::time::Duration::from_secs(5));
    match beatplayer.play_beat() {
        Ok(_) => println!("Started the beatplayer again while running, this should not happen!"),
        Err(e) => println!("Not possible to start the beatplayer while a beat is playing, good. Following error was reported:\n{}", e),
    }

    println!("Stopping thread");
    beatplayer.stop();
    println!("Thread has stopped");
}
