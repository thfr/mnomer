# Mnomer

Mnomer is a metronom application written in Rust.
It is an enhanced version of the C++ project [mnome](https://github.com/thfr/mnome).
The main purpose is to experiment with and train myself in the Rust programming language.

It should work on Linux, macOS and Windows.

Current version is [0.2.1](https://github.com/thfr/mnomer/releases/tag/0.2.1).

## Features

* a simple [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop)
* 3 beat types: Accent `!`, Beat `+` and Pause `.`
* start/stop with ENTER key
* pitch, bpm, beat pattern and beat value changeable
* help

## Usage

Following commands are implemented:

* `start`
* `stop`
* `bpm <number>`, based on the beat value 1/4
* `pitch <accent> <normal>`
* `pattern <pattern>` with `<pattern>` adhering to `[!|+|\.]*`
* `value <beat value>`, defaults to `4` which means the beat is 1/4
* `help [<command>]`, shows the commands when no additional command is given or the help for a specific command
* `quit`, `exit` or CTRL+C exits the application

```plain
♩♩♩♩: help
Known commands: "help" <ENTER> "start" "pattern" "pitch" "quit" "value" "exit" "stop" "bpm"
♩♩♩♩: help pitch
"pitch <accentuated beat pitch> <normal beat pitch>"
  pitches must should within [20; 20k]Hz
♩♩♩♩: help value
"value <note value subdivision for beat pattern>"
  defaults to 4 (beat has a 1/4 note value which is the base for the bpm value)
♩♩♩♩: help bpm
"bpm <value>" where <value> >= 1
  This value is based on a beat value of 4 (1/4 note value)
♩♩♩♩: help pattern
"pattern <pattern>"
  <pattern> must be in the form of `[!|+|.]*`
  `!` = accentuated beat  `+` = normal beat  `.` = pause
♩♩♩♩:
pattern: !+++  value: 1/4 bpm: 100  !: 587.330Hz  +:440.000Hz
```
