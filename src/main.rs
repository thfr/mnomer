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

    let mut beatplayer = BeatPlayer::new(80, AudioSignal::generate_tone(440.0, 0.05, 1), sine.clone(), vec![true]);

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
