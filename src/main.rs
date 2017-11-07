#![allow(dead_code)]

extern crate portmidi;
extern crate monochord;
extern crate rustyline;

use std::io::{Read, BufReader};

#[derive(Debug)]
enum Message {
    NoteOn(u8, (i8, i16), u8),
    NoteOff(u8, (i8, i16)),
    Edo(u16),
    Key(i16),
    Bpm(u16),
    Lpb(u16),
    Wait,
    Stop,
}

struct Backend {
    midi_out: portmidi::OutputPort,
    messages: ::std::sync::mpsc::Receiver<Message>,
    period: u16,
    key: i16,
    bpm: u16,
    lpb: u16,
}

impl Backend {
    fn spawn(port: portmidi::OutputPort) -> ::std::sync::mpsc::Sender<Message> {
        let (tx, rx) = ::std::sync::mpsc::channel();
        ::std::thread::spawn(move || {
            let mut backend = Backend {
                midi_out: port,
                messages: rx,
                period: 31,
                key: 0,
                bpm: 120,
                lpb: 4,
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
                    Wait => {
                        let lps = 60_000.0 / (self.bpm as f64 * self.lpb as f64);
                        let dt = ::std::time::Duration::from_millis(lps as u64);
                        ::std::thread::sleep(dt);

                        match self.messages.try_recv() {
                            Ok(Stop) => {
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
            _ => (),
        }
    }

    fn note_on(&mut self, ch: u8, note: (i8, i16), velocity: u8) {
        let note = 60 + self.key as i32 +
            self.period as i32 * note.0 as i32 +
            note.1 as i32;
        if note <= 127 && note >= 0 {
            let msg = portmidi::MidiMessage {
                status: 0x90 + (ch as u8),
                data1: note as u8,
                data2: velocity,
            };
            drop(self.midi_out.write_message(msg));
        }
    }

    fn note_off(&mut self, ch: u8, note: (i8, i16), velocity: u8) {
        let note = 60 + self.key as i32 +
            self.period as i32 * note.0 as i32 +
            note.1 as i32;
        if note <= 127 && note >= 0 {
            let msg = portmidi::MidiMessage {
                status: 0x80 + (ch as u8),
                data1: note as u8,
                data2: velocity,
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

pub fn tokenize(string: String) -> Vec<Token> {
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
            Some('-') => {
                let la = chs.next();
                match la {
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
}

impl Interpreter {
    fn read_integer(&mut self) -> Result<i64, ()> {
        match self.stack.pop() {
            Some(Token::Integer(i)) => Ok(i),
            _ => Err(()),
        }
    }

    fn read_note(&mut self) -> Result<(i8, i16), ()> {
        match self.stack.pop() {
            Some(Token::Note(o, n)) => Ok((o, n)),
            _ => Err(()),
        }
    }

    fn exec(&mut self, line: String) -> Result<(), ()> {
        let toks = tokenize(line);
        for t in toks.into_iter() {
            match t {
                Token::Word(word) => self.exec_word(&word)?,
                v => self.stack.push(v),
            }
        }
        Ok(())
    }

    fn exec_word(&mut self, word: &str) -> Result<(), ()> {
        match word {
            "+" => {
                let vel = self.read_integer()?;
                let note = self.read_note()?;
                let ch = self.read_integer()?;

                drop(self.backend.send(Message::NoteOn(ch as u8, note, vel as u8)));
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
            }
            "w" => {
                drop(self.backend.send(Message::Wait));
            },
            _ => (),
        }
        Ok(())
    }
}

fn main() {
    let midi = portmidi::PortMidi::new().unwrap();
    let out = midi.default_output_port(1024).unwrap();

    let backend = Backend::spawn(out);
    let mut int = Interpreter {
        stack: vec![],
        backend,
        file: None,
        lines: vec![],
    };
    let mut rl: rustyline::Editor<()> = rustyline::Editor::new();
    rl.set_history_max_len(1024);

    loop {
        let line = rl.readline("? ");
        match line {
            Ok(line) => {
                int.exec(line);
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

        let string = "1d0,64+ 2d7,64+ 1d0-".to_string();
        let tokens = tokenize(string);
        assert_eq!(&*tokens, &[
            Integer(1), Note(0, 0), Integer(64), Word("+".to_string()),
            Integer(2), Note(0, 7), Integer(64), Word("+".to_string()),
            Integer(1), Note(0, 0), Word("-".to_string())
        ]);
    }
}