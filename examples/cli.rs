use std::io::{stdin, BufRead, BufReader, Lines};

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
        if self.cs.is_empty() {
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
            self.cs.pop().and_then(|c| Some(Ok(c)))
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
    let br = Chars::new(BufReader::new(stdin()));
    let mut gp = gcode::Parser::new(br);
    loop {
        let tok = gp.next();
        if tok.is_none() {
            continue;
        }
        let tok = tok.unwrap();

        println!("{:?}", tok);
    }
}
