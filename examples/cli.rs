use futures::stream;
use futures_executor::block_on;
use std::io::Read;

use gcode::Parser;

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Parse(gcode::Error),
}
impl From<gcode::Error> for Error {
    fn from(f: gcode::Error) -> Self {
        Self::Parse(f)
    }
}
fn main() {
    block_on(async {
        let mut parser = Parser::new(stream::iter(
            std::io::stdin().bytes().map(|res| res.map_err(Error::Io)),
        ));

        loop {
            if let Some(res) = parser.next().await {
                println!("{:?}", res);
            } else {
                break;
            }
        }
    });
}
