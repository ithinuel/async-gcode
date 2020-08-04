use futures::{future, stream, StreamExt};
use futures_executor::block_on;
use std::io::{stdin, Read};

use gcode::Parser;

fn main() {
    block_on(async {
        let parser = Parser::new(stream::iter(stdin().bytes().filter_map(
            |input| match input {
                Ok(input) => Some(input),
                Err(e) => {
                    println!("{:?}", e);
                    None
                }
            },
        )));
        stream::unfold(parser, Parser::next)
            .for_each(|gcode| {
                println!("{:?}", gcode);
                future::ready(())
            })
            .await;
        println!("Done");
    });
}
