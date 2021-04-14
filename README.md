
# Mnomer

Mnome is a metronom application written in Rust.
It is mainly an enhanced copy of [mnome](https://github.com/thfr/mnome),
a C++ project of mine.
The main purpose is to experiment with and train myself in the Rust programming language.

# Usage

Following commands are implemented: `start`, `stop`, `bpm <number>`
and `pattern <pattern>` with `<pattern>` adhering to `[!|+|\.]*`

```txt
♩♩♩♩: <ENTER>
bpm: 100, pattern: BeatPattern([Accent, Beat, Beat, Beat]), playing: true

♩♩♩♩: bpm 80
bpm: 80, pattern: BeatPattern([Accent, Beat, Beat, Beat]), playing: true

♩♩♩♩: pattern !+.+
bpm: 80, pattern: BeatPattern([Accent, Beat, Pause, Beat]), playing: true

♩♩♩♩: start
bpm: 80, pattern: BeatPattern([Accent, Beat, Pause, Beat]), playing: true

♩♩♩♩: stop
bpm: 80, pattern: BeatPattern([Accent, Beat, Pause, Beat]), playing: false

♩♩♩♩: quit
```
