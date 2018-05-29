extern crate rand;

use std::io::prelude::*;
use std::io::{self, Write};
use std::net::TcpStream;
use std::env;
use std::fmt;
use rand::Rng;
use std::str;

#[ derive( Clone ) ]
struct Node {
    ip: String,
    port: String,
}

impl fmt::Display for Node {
    fn fmt( &self, fmt: &mut fmt::Formatter ) -> fmt::Result {
        fmt.write_str( &format!( "{}:{}", self.ip, self.port ) )?;
        Ok( () )
    }
}

#[ derive( Clone ) ]
struct Request<'a> {
    node: &'a Node,
    request_string: String,
}

#[ derive( Clone ) ]
struct Response<'a> {
    status: u8,
    message: &'a [u8],
}

fn verify_response( response: Response ) {
    if response.status != 0 {
        eprintln!( "ERROR: {}", str::from_utf8( response.message ).unwrap() );
        panic!( "bad request, see above error" );
    }
}

fn perform_request<'a>( request: Request, response_buffer: &'a mut Vec<u8> )
                        -> Response<'a> {
    let mut stream = TcpStream::connect( request.node.to_string() ).unwrap();
    let request_bytes: &[u8] = request.request_string.as_bytes();
    
    let _ = stream.write( request_bytes );
    stream.shutdown( std::net::Shutdown::Write ).unwrap();

    let _ = stream.read_to_end( response_buffer );

    Response {
        status: response_buffer[ 0 ],
        message: &response_buffer[ 1.. ],
    }
}

fn request_length( node: &Node, file_name: &String ) -> u16 {
    let request = Request {
        node: node,
        request_string: format!( "{}:(LENGTH)", file_name ),
    };
    let response_buffer = &mut Vec::new();
    let response = perform_request( request, response_buffer );
    verify_response( response.clone() );

    str::from_utf8( response.message ).unwrap().parse::<u16>().unwrap()
}

fn request_file<'a>( node: &Node, file_name: &String,
                 start_offset: u16, end_offset: u16,
                 response_buffer: &'a mut Vec<u8> ) -> Option< &'a [u8] > {
    let request = Request {
        node: node,
        request_string: format!( "{}:(READ,{},{})", file_name,
                                      start_offset, end_offset ),
    };
    let response = perform_request( request, response_buffer );

    if response.status == 0 {
        Some( response.message )
    } else {
        None
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[ 1 ];

    // retrieve /file_store.yaml from one of the known nodes
    // (stored in /usr/local/rustfs/nodes.yaml)
    // TODO
    
    // find nodes that have this file
    // TODO
    // TODO return error if no nodes have the file
    let active_nodes = &mut Vec::new();
    for _ in 0..42 {
        active_nodes.push(
            Node { ip: "127.0.0.1".to_string(), port: "2345".to_string() } );
    }

    // get the file length randomly from one of the nodes
    let rand_nodes = active_nodes.clone();
    let node = rand::thread_rng().choose( &rand_nodes ).unwrap();
    let file_length = request_length( node, file_name );

    // if there are more active nodes than bytes in the file, need to randomly select
    // file_length number of nodes to use, chunk size will be one byte
    // TODO

    // split the file into even chunks based on how many nodes there are,
    // then request the file chunks from the nodes
    let file_contents = &mut Vec::new();
    let num_nodes = active_nodes.len() as u16;
    let chunk_size: u16 = std::cmp::max( file_length / num_nodes, 1 );
    let mut start_offset = 0;
    let mut end_offset = 0;
    let mut count = 0;
    for node in active_nodes {
        end_offset += chunk_size;
        if end_offset > file_length {
            break;
        }
        if count == num_nodes - 1 {
            end_offset = file_length;
        }
        let response_buffer = &mut Vec::new();
        let file_chunk_option = request_file( node, file_name, start_offset, end_offset,
                                       response_buffer );
        let file_chunk = match file_chunk_option {
            Some( file_chunk ) => file_chunk,
            None => panic!( "Chunk failed, need to retry" ), // TODO
        };
        file_contents.extend_from_slice( file_chunk );
        start_offset = end_offset;
        count += 1;
    }

    // retry on failed chunks from different nodes
    // TODO

    // output the file to stdout
    io::stdout().write( file_contents ).unwrap();
}
