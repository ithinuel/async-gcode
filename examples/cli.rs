use gcode::GCode;
//use std::collections::BTreeMap;
use std::io::{stdin, BufRead, BufReader, Lines};

/*
use mio::{Ready, Poll, PollOpt, Token};
use mio::unix::EventedFd;

use std::os::unix::io::AsRawFd;
use std::net::TcpListener;

// Bind a std listener
let poll = Poll::new()?;

// Register the listener
poll.register(&EventedFd(&stdin().as_raw_fd()),
             Token(0), Ready::readable(), PollOpt::edge())?;
*/
struct Chars<B> {
    br: B,
    cs: String,
    stopped: bool,
}

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Parser(gcode::Error),
}
impl From<gcode::Error> for Error {
    fn from(f: gcode::Error) -> Error {
        Error::Parser(f)
    }
}

impl<B> Iterator for Chars<B>
where
    B: Iterator<Item = Result<String, std::io::Error>>,
{
    type Item = Result<char, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped {
            return None;
        }
        let mut out = None;
        if self.cs.len() == 0 {
            match self.br.next() {
                Some(Err(e)) => {
                    self.stopped = true;
                    out = Some(Err(Error::Io(e)));
                }
                Some(Ok(mut line)) => {
                    line.push('\r');
                    self.cs = line.chars().rev().collect();
                }
                _ => {}
            }
        }
        if out.is_some() {
            out
        } else {
            self.cs.pop().map_or(None, |c| Some(Ok(c)))
        }
    }
}

impl<B: BufRead> Chars<Lines<B>> {
    pub fn new(d: B) -> Chars<Lines<B>> {
        Chars {
            br: d.lines(),
            cs: String::new(),
            stopped: false,
        }
    }
}

fn main() {
    //let mut params = BTreeMap::new();

    let br = Chars::new(BufReader::new(stdin()));
    let mut gp = gcode::Parser::new(br);
    loop {
        let tok = gp.next();
        if tok.is_none() {
            continue;
        }
        let tok = tok.unwrap();
        println!("{:?}", tok);
        match tok {
            Ok(GCode::Execute) => {}
            //Ok(Token::Word(_w, _c)) => {}
            //Ok(Token::ParamSetting(lhs, rhs)) => {
            //let lhs = lhs.round() as u32;
            //params.insert(lhs, rhs);
            //}
            //            Ok(Token::Comment(_comment)) => {}
            Ok(_) => {}
            Err(_e) => {}
        }
    }
}
