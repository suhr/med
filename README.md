# MED: The Ed of Music Trackers

Med is a simple tool that reads commands and emits MIDI messages. When it reads commands from a file, it executes each line with a delay, working as a music tracker.

Commands are postfix, so values are listed before the command. Spaces are ignored, as well as the `,` symbol. Note syntax is `[a-h]\d+`. So `0d0,64+` is the same as `0 d0 64 +`.

Supported commands:

- (`ch`, `note`, `vel`) `+`: Emits `NoteOn` on channel `chan` with note `note` and velocity `vel`
- (`ch`, `note`) `-`; Emits `NoteOff` on channel `chan` with note `note`
- (`ch`, `note`, `vel`) `.`: Emits `NoteOn` and postpones `NoteOff` to the next line
- (`key`) `key`: Sets the key, which shifts 0th note by `key` steps
- (`edo`) `edo`: Sets the [EDO](http://xenharmonic.wikispaces.com/EDO). Default is 12
- (`bpm`) `bpm`: Sets the beat per minute
- (`lpb`) `lpb`: Sets the lines per beat
- `w`: Wait one line time
- (`from`, `to`) `p`: Play file lines, from `from` to `to`. Each line is played with a delay, like in a music tracker
- `s`: Stop playing

## Note syntax

Note symbol looks like `c7` or `d0`. The symbol, which can be from ‘a’ to ‘h’ is the octave. The number, which can be any reasonably sized integer, is the number of steps. That number is different in different EDOs, for example, a perfect fifth is 7 steps wide in 12edo but 18 steps wide in 31edo.

`d0` is always MIDI note 60, or C-4 in 12edo.
