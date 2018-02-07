# MED: The Ed of Music Trackers

Med is a simple tool that reads commands and emits MIDI messages. It can read commands line from the stdin, or from a file.  When it reads commands from a file, it executes each line with a delay, working as a music tracker.

Commands are postfix, so values are listed before the command. Spaces are ignored, as well as the `,` symbol. Note syntax is `[a-h]\d+`. So `0d0,64+` is the same as `0 d0 64 +`. `_` is null, it used in place of optional arguments.

Everything in a line after `--` is a comment and is ignored by the interpreter.

Supported commands (arguments in `[]` are optional):

- (`ch`, `note`, [`vel`]) `+`: Emits `NoteOn` on channel `chan` with note `note` and velocity `vel`
- (`ch`, `note`) `-`; Emits `NoteOff` on channel `chan` with note `note`
- (`ch`, `note`, [`vel`]) `.`: Emits `NoteOn` and postpones `NoteOff` to the next line
- (`key`) `key`: Sets the key, which shifts 0th note by `key` steps
- (`edo`) `edo`: Sets the [EDO](http://xenharmonic.wikispaces.com/EDO). Default is 12
- (`bpm`) `bpm`: Sets the beat per minute
- (`lpb`) `lpb`: Sets the lines per beat
- (`n`) `w`: Wait n line times
- (`from`, [`to`]) `p`: Play file lines, from `from` to `to`. Each line is played with a delay, like in a music tracker
- `s`: Stop playing

When you open a file with med, it immediately executes (without delay) lines before the first empty line. After that, it shows a cli allowing you to enter commands.

Use `p` to play the file and `s` to stop playing.

## Installation

1. Install [Rust](https://www.rust-lang.org/)
2. `cargo install --git https://github.com/suhr/med.git`


## Note syntax

Note symbol looks like `c7` or `d0`. The symbol, which can be from ‘a’ to ‘h’ is the octave. The number, which can be any reasonably sized integer, is the number of steps. That number is different in different EDOs, for example, a perfect fifth is 7 steps wide in 12edo but 18 steps wide in 31edo.

`d0` is MIDI note 60, or C-4 in 12edo.
