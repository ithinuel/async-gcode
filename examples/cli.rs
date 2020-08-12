use futures::{future, stream, StreamExt};
use futures_executor::block_on;
use std::io::{stdin, Read};

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
            stdin().bytes().map(|res| res.map_err(Error::Io)),
        ));

        stream::unfold(
            &mut parser,
            |p| async move { p.next().await.map(|w| (w, p)) },
        )
        .for_each(|gcode| {
            println!("{:?}", gcode);
            future::ready(())
        })
        .await;
        println!("Done");
    });
}
