extern crate tokio;

use std::error::Error;
use std::fs::File;
use std::path::Path;
use tokio::prelude::*;
use tokio::io;
use tokio::net::TcpListener;

fn main() {
    let address = "127.0.0.1:2345".parse().unwrap();
    let listener = TcpListener::bind( &address )
        .expect( "cannot bind TCP listener" );

    // handle incoming connections
    let server = listener.incoming()
        .map_err( | e | eprintln!( "accept failed = {:?}", e ) )
        .for_each( | socket | {
            // split socket into reader and writer
            let ( reader, writer ) = socket.split();

            let buf: Vec<u8> = Vec::new();

            let handle_conn = io::read_to_end( reader, buf )
                .and_then( | ( _, buf ) | {
                    let file_name = std::str::from_utf8(
                        &buf[ .. ] ).unwrap();

                    let path = Path::new( file_name );
                    let mut file_contents = String::new();

                    // open file and read from it
                    let file_oper = match File::open( &file_name ) {
                        Err( why ) => Err( format!(
                            "couldn't open {}: {}\n", path.display(),
                            why.description() ) ),
                        Ok( mut file ) => {
                            // read file contents into string
                            match file.read_to_string( &mut file_contents ) {
                                Err( why ) => Err( format!(
                                    "couldn't read {}: {}\n", path.display(),
                                    why.description() ) ),
                                Ok( _ ) => Ok( () ),
                            }
                        },
                    };

                    // write file contents (or error) into writer socket
                    match file_oper {
                        Err( why ) => {
                            eprint!( "{}", why );
                            io::write_all( writer, why )
                        },
                        Ok( _ ) => io::write_all( writer, file_contents ),
                    }
                } )
                .then( | _ | {
                    Ok( () )
                } );

            // spawn future concurrently
            tokio::spawn( handle_conn );

            Ok( () )
        } );

    // start tokio runtime
    tokio::run( server );
}
