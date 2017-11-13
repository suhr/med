#![allow(dead_code)]

extern crate portmidi;
extern crate monochord;
extern crate rustyline;
extern crate clap;

use std::io::{Read, BufReader, BufRead};
use std::path::PathBuf;

#[derive(Debug)]
enum Message {
    NoteOn(u8, (i8, i16), u8),
    NoteOff(u8, (i8, i16)),
    Edo(u16),
    Key(i16),
    Bpm(u16),
    Lpb(u16),
    Wait(u16),
    Stop,
}

struct Backend {
    midi_out: portmidi::OutputPort,
    messages: ::std::sync::mpsc::Receiver<Message>,
    period: u16,
    key: i16,
    bpm: u16,
    lpb: u16,
    notes: Vec<(u8, (i8, i16))>
}

impl Backend {
    fn spawn(port: portmidi::OutputPort) -> ::std::sync::mpsc::Sender<Message> {
        let (tx, rx) = ::std::sync::mpsc::channel();
        ::std::thread::spawn(move || {
            let mut backend = Backend {
                midi_out: port,
                messages: rx,
                period: 12,
                key: 0,
                bpm: 120,
                lpb: 4,
                notes: vec![],
            };

            backend.run()
        });

        tx
    }

    fn run(&mut self) {
        use Message::*;
        loop {
            let msgs: Vec<Message> = self.messages.try_iter().collect();
            for m in msgs {
                match m {
                    Wait(t) => {
                        let lps = 60_000.0 / (self.bpm as f64 * self.lpb as f64);
                        let dt = ::std::time::Duration::from_millis(lps as u64 * t as u64);
                        ::std::thread::sleep(dt);

                        match self.messages.try_recv() {
                            Ok(Stop) => {
                                self.all_notes_off();
                                break
                            },
                            Ok(msg) => self.run_msg(msg),
                            _ => (),
                        }
                    },
                    msg => self.run_msg(msg),
                }
            }

            let dt = ::std::time::Duration::from_millis(8);
            ::std::thread::sleep(dt);
        }
    }

    fn run_msg(&mut self, msg: Message) {
        use Message::*;
        match msg {
            NoteOn(ch, note, vel) => self.note_on(ch, note, vel),
            NoteOff(ch, note) => self.note_off(ch, note, 64),
            Key(key) => self.key = key,
            Bpm(bpm) => self.bpm = bpm,
            Lpb(lpb) => self.lpb = lpb,
            Edo(edo) => self.period = edo,
            Stop => self.all_notes_off(),
            _ => (),
        }
    }

    fn note_on(&mut self, ch: u8, note: (i8, i16), velocity: u8) {
        let key = 60 + self.key as i32 +
            self.period as i32 * note.0 as i32 +
            note.1 as i32;

        if key <= 127 && key >= 0 {
            self.notes.push((ch,note));

            let msg = portmidi::MidiMessage {
                status: 0x90 + (ch as u8),
                data1: key as u8,
                data2: velocity,
            };
            drop(self.midi_out.write_message(msg));
        }
    }

    fn note_off(&mut self, ch: u8, note: (i8, i16), velocity: u8) {
        let key = 60 + self.key as i32 +
            self.period as i32 * note.0 as i32 +
            note.1 as i32;

        if key <= 127 && key >= 0 {
            self.notes.retain(|n| *n != (ch, note));

            let msg = portmidi::MidiMessage {
                status: 0x80 + (ch as u8),
                data1: key as u8,
                data2: velocity,
            };
            drop(self.midi_out.write_message(msg));
        }
    }

    fn all_notes_off(&mut self) {
        for (ch, note) in self.notes.drain(..) {
            let key = 60 + self.key as i32 +
                self.period as i32 * note.0 as i32 +
                note.1 as i32;

            let msg = portmidi::MidiMessage {
                status: 0x80 + (ch as u8),
                data1: key as u8,
                data2: 64,
            };
            drop(self.midi_out.write_message(msg));        
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Note(i8, i16),
    String(String),
    Integer(i64),
    Word(String),
    Null,
}

fn is_digit(d: char) -> bool {
    match d {
        '0'...'9' => true,
        _ => false,
    }
}

fn is_letter(l: char) -> bool {
    match l {
        'A'...'Z' | 'a'...'z' => true,
        _ => false,
    }
}

fn octave_num(l: char) -> Option<i8> {
    match l {
        l @ 'a'...'h' => Some(l as i8 - 'd' as i8),
        _ => None,
    }
}

fn read_number(first: char, chs: &mut ::std::str::Chars) -> (String, Option<char>) {
    let mut num = String::new();
    num.push(first);

    loop {
        match chs.next() {
            Some(d) if is_digit(d) => num.push(d),
            c => {
                return (num, c)
            },
        }
    }
}

pub fn tokenize(string: &str) -> Vec<Token> {
    let mut chs = string.chars();
    let mut toks = vec![];

    let mut ch = chs.next();
    loop {
        match ch {
            Some('"') => unimplemented!(),
            Some(d) if is_digit(d) => {
                let (num, rem) = read_number(d, &mut chs);
                toks.push(Token::Integer(num.parse().expect("INTEGER FAIL")));

                ch = rem;
                continue
            },
            Some(l) if is_letter(l) => {
                let la = chs.next();
                match (octave_num(l), la) {
                    (Some(n), Some(la)) if is_digit(la) => {
                        let (num, la) = read_number(la, &mut chs);
                        let note = num.parse().expect("NOTE FAIL");
                        toks.push(Token::Note(n, note));

                        ch = la;
                        continue
                    },
                    _ => {
                        let mut word = String::new();
                        word.push(l);

                        let mut la = la;
                        loop {
                            match la {
                                Some(l) if is_letter(l) => {
                                    word.push(l)
                                },
                                c => {
                                    toks.push(Token::Word(word));
                                    la = c;
                                    break
                                }
                            }

                            la = chs.next();
                        }

                        ch = la;
                        continue
                    },
                }
            },
            Some(' ') | Some('\n') | Some('\t') | Some(',') => (),
            Some('_') => {
                toks.push(Token::Null);
            }
            Some('-') => {
                let la = chs.next();
                match la {
                    Some('-') => {
                        break
                    },
                    Some(d) if is_digit(d) => {
                        let (num, la) = read_number(d, &mut chs);
                        let n: i64 = num.parse().expect("MINUS FAIL");
                        toks.push(Token::Integer(-n));

                        ch = la;
                        continue
                    }
                    _ => {
                        toks.push(Token::Word("-".to_string()));
                        ch = la;
                        continue
                    },
                }
            },
            Some(c) => {
                let mut word = String::new();
                word.push(c);
                toks.push(Token::Word(word));
            },
            None => break,
        };

        ch = chs.next()
    }

    toks
}

struct Interpreter {
    stack: Vec<Token>,
    backend: ::std::sync::mpsc::Sender<Message>,
    file: Option<::std::path::PathBuf>,
    lines: Vec<String>,
    postponed: Vec<Message>,
}

impl Interpreter {
    fn init(&mut self) {
        if let Some(file) = self.file.clone() {
            self.lines = open_file(&file);

            for l in &*self.lines.clone() {
                if l == "" { break }
                println!("{}", l);
                self.exec(l);
            }
        }
    }
    fn read_integer(&mut self) -> Result<i64, ()> {
        match self.stack.pop() {
            Some(Token::Integer(i)) => Ok(i),
            _ => Err(()),
        }
    }

    fn read_option_integer(&mut self) -> Result<Option<i64>, ()> {
        match self.stack.pop() {
            Some(Token::Integer(i)) => Ok(Some(i)),
            Some(Token::Null) => Ok(None),
            _ => Err(()),
        }
    }

    fn read_note(&mut self) -> Result<(i8, i16), ()> {
        match self.stack.pop() {
            Some(Token::Note(o, n)) => Ok((o, n)),
            _ => Err(()),
        }
    }

    fn exec(&mut self, line: &str) -> Result<(), ()> {
        let toks = tokenize(line);
        for t in toks.into_iter() {
            match t {
                Token::Word(word) => self.exec_word(&word)?,
                v => self.stack.push(v),
            }
        }
        Ok(())
    }

    fn read_file(&mut self) -> Result<(), ()> {
        if let Some(ref file) = self.file {
            let file = ::std::fs::File::open(file).map_err(|_| ())?;
            let reader = BufReader::new(file);
            
            let mut lines: Vec<String> = vec![];
            for l in reader.lines() {
                lines.push(l.map_err(|_| ())?)
            };

            self.lines = lines
        };

        Ok(())
    }

    fn exec_word(&mut self, word: &str) -> Result<(), ()> {
        match word {
            "+" => {
                let vel = self.read_option_integer()?.unwrap_or(64);
                let note = self.read_note()?;
                let ch = self.read_integer()?;

                drop(self.backend.send(Message::NoteOn(ch as u8, note, vel as u8)));
            },
            "." => {
                let vel = self.read_option_integer()?.unwrap_or(64);
                let note = self.read_note()?;
                let ch = self.read_integer()?;

                drop(self.backend.send(Message::NoteOn(ch as u8, note, vel as u8)));
                self.postponed.push(Message::NoteOff(ch as u8, note))
            },
            "-" => {
                let note = self.read_note()?;
                let ch = self.read_integer()?;

                drop(self.backend.send(Message::NoteOff(ch as u8, note)));
            },
            "key" => {
                let key = self.read_integer()?;

                drop(self.backend.send(Message::Key(key as i16)));
            },
            "edo" => {
                let edo = self.read_integer()?;

                drop(self.backend.send(Message::Edo(edo as u16)));
            },
            "bpm" => {
                let bpm = self.read_integer()?;
                drop(self.backend.send(Message::Bpm(bpm as u16)));
            },
            "lps" => {
                let lpb = self.read_integer()?;
                drop(self.backend.send(Message::Lpb(lpb as u16)));
            },
            "w" => {
                let time = self.read_integer()?;
                drop(self.backend.send(Message::Wait(time as u16)));
            },
            "p" => {
                self.read_file()?;

                let to = self.read_option_integer()?.unwrap_or(self.lines.len() as i64);
                let from = self.read_integer()? - 1;

                let lines = self.lines.get(from as usize .. to as usize).ok_or(())?.to_owned();
                for l in lines {
                    self.exec(&l);
                    drop(self.backend.send(Message::Wait(1)));

                    for m in self.postponed.drain(..) {
                        drop(self.backend.send(m));
                    }
                }
            },
            "s" => {
                drop(self.backend.send(Message::Stop));
            }
            "r" => {
                self.read_file()?
            }
            _ => (),
        }
        Ok(())
    }
}

fn open_file(path: &std::path::PathBuf) -> Vec<String> {
    let name = path.to_str().unwrap();
    let file = ::std::fs::File::open(&path)
        .expect(&*format!("Failed to open {}", name));
    let reader = BufReader::new(file);
    
    let mut lines: Vec<String> = vec![];
    for l in reader.lines() {
        lines.push(l.expect(&*format!("Failed to read {}", name)))
    };

    lines
}

fn main() {
    let matches = clap::App::new("med")
        .version(env!("CARGO_PKG_VERSION"))
        .about("The Ed of music trackers")
        .arg(
            clap::Arg::with_name("file")
            .help("MED file")
            .index(1)
        )
        .get_matches();

    let file = matches.value_of("file").map(::std::path::PathBuf::from);
    let midi = portmidi::PortMidi::new().unwrap();
    let out = midi.default_output_port(1024).unwrap();

    let backend = Backend::spawn(out);
    let mut int = Interpreter {
        stack: vec![],
        backend,
        file,
        lines: vec![],
        postponed: vec![],
    };

    int.init();

    let mut rl: rustyline::Editor<()> = rustyline::Editor::new();
    rl.set_history_max_len(1024);

    loop {
        let line = rl.readline("? ");
        match line {
            Ok(line) => {
                rl.add_history_entry(&line);
                int.exec(&line);
            },
            _ => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tokenize() {
        use Token::*;

        let string = "1d0,64+ 2d7,64+ 1d0-  -- f1. Hello I'm a comment!";
        let tokens = tokenize(string);
        assert_eq!(&*tokens, &[
            Integer(1), Note(0, 0), Integer(64), Word("+".to_string()),
            Integer(2), Note(0, 7), Integer(64), Word("+".to_string()),
            Integer(1), Note(0, 0), Word("-".to_string())
        ]);
    }
}
