# Mnomer

Mnomer is a metronom application written in Rust.
It is an enhanced copy of [mnome](https://github.com/thfr/mnome),
a C++ project of mine.
The main purpose is to experiment with and train myself in the Rust programming language.

It should work on Linux, macOS and Windows.

Current version is [0.1.0](https://github.com/thfr/mnomer/releases/tag/0.1.0).

## Features

* a simple [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop)
* 3 beat types: Accent, Beat and Pause
* start/stop with ENTER key
* pitch, bpm and beat pattern change during playback

## Usage

Following commands are implemented: `start`, `stop`, `bpm <number>`,
`pitch <accent> <normal>` and `pattern <pattern>` with `<pattern>` adhering to `[!|+|\.]*`.

```txt
♩♩♩♩:
bpm:  100, pattern: BeatPattern([Accent, Beat, Beat, Beat]), accent: 587.33Hz, normal: 440.00Hz, playing: true

♩♩♩♩: bpm 80
bpm:   80, pattern: BeatPattern([Accent, Beat, Beat, Beat]), accent: 587.33Hz, normal: 440.00Hz, playing: true

♩♩♩♩: pattern !+.+
bpm:   80, pattern: BeatPattern([Accent, Beat, Pause, Beat]), accent: 587.33Hz, normal: 440.00Hz, playing: true

♩♩♩♩: start
bpm:   80, pattern: BeatPattern([Accent, Beat, Pause, Beat]), accent: 587.33Hz, normal: 440.00Hz, playing: true

♩♩♩♩: stop
bpm:   80, pattern: BeatPattern([Accent, Beat, Pause, Beat]), accent: 587.33Hz, normal: 440.00Hz, playing: false

♩♩♩♩: bpm
Error in command "bpm": No bpm value supplied
Command usage: "bpm <value>" where <value> >= 1

♩♩♩♩: help
"help" command unknown
Following commands are defined:
<ENTER>
"start"
"stop"
"bpm"
"pattern"
"pitch"

♩♩♩♩: pitch 800 600
bpm:   80, pattern: BeatPattern([Accent, Beat, Pause, Beat]), accent: 800.00Hz, normal: 600.00Hz, playing: false

♩♩♩♩: quit
```
