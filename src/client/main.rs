use std::io::prelude::*;
use std::io::{self, Write};
use std::net::TcpStream;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[1];

    let mut stream = TcpStream::connect( "127.0.0.1:2345" ).unwrap();

    let request_string = format!( "{}:(READ)", file_name );
    let request: &[u8] = request_string.as_bytes();

    let response: &mut Vec<u8> = &mut Vec::new();

    let _ = stream.write( request );
    stream.shutdown( std::net::Shutdown::Write ).unwrap();

    let _ = stream.read_to_end( response );

    io::stdout().write( response ).unwrap();
}
