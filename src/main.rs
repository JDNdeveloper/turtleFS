extern crate tokio;

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

            let handle_conn = io::write_all( writer, "hello world\n" )
                .and_then( | ( writer, _ ) | {
                    io::copy( reader, writer )
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
