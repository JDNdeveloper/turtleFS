use std::io::prelude::*;
use std::net::TcpStream;

fn main() {
    let mut stream = TcpStream::connect( "127.0.0.1:2345" ).unwrap();

    let request: &[u8] = b"/Users/jayden/rust_projects/file_server/\
                           src/example_files/hello.txt:(READ)";
    let response: &mut Vec<u8> = &mut Vec::new();

    let _ = stream.write( request );
    stream.shutdown( std::net::Shutdown::Write ).unwrap();
    let _ = stream.read_to_end( response );

    let s = String::from_utf8_lossy( response );

    print!( "{}", s );
}
