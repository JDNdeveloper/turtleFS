extern crate rand;
extern crate yaml_rust;
extern crate crc;

use std::io::prelude::*;
use std::io::{self, Write};
use std::net::TcpStream;
use std::env;
use std::fmt;
use std::fs::File;
use rand::Rng;
use std::str;
use yaml_rust::{Yaml,YamlLoader,yaml};
use crc::crc32;

#[ derive( Clone, Debug ) ]
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

fn retrieve_root_nodes() -> Vec< Node > {
    let mut root_nodes = Vec::new();

    let mut f = File::open( "/usr/local/rustfs/nodes.yaml" ).unwrap();
    let mut contents = String::new();
    f.read_to_string( &mut contents ).unwrap();

    let node_yaml = YamlLoader::load_from_str( &contents ).unwrap();
    for node_string in node_yaml[ 0 ].as_vec().unwrap() {
        let node_info: Vec< &str > =
            node_string.as_str().unwrap().split( ":" ).collect();
        root_nodes.push(
            Node { ip: node_info[ 0 ].to_string(),
                   port: node_info[ 1 ].to_string() } );
    }

    return root_nodes;
}

fn retrieve_active_nodes( file_name: &str, file_store_map: &yaml::Hash )
                          -> ( u32, Vec< Node > ) {
    let mut active_nodes = Vec::new();
    let mut checksum = 0;
    for ( key_file, key_info ) in file_store_map {
        if key_file.as_str().unwrap() == file_name {
            let key_info_hash = key_info.as_hash().unwrap();

            // get checksum
            let checksum_str = key_info_hash.get(
                &Yaml::from_str( "checksum" ) ).unwrap().as_str().unwrap();
            checksum = u32::from_str_radix( checksum_str, 16 ).unwrap();

            // get nodes
            let val_node_list = key_info_hash.get(
                &Yaml::from_str( "nodes" ) ).unwrap();
            for node_string in val_node_list.as_vec().unwrap() {
                let node_info: Vec< &str > =
                    node_string.as_str().unwrap().split( ":" ).collect();
                active_nodes.push(
                    Node { ip: node_info[ 0 ].to_string(),
                           port: node_info[ 1 ].to_string() } );
            }
        }
    }
    if active_nodes.len() == 0 {
        panic!( "ERROR: no active nodes for file {}", file_name );
    }
    return ( checksum, active_nodes );
}

fn perform_request<'a>( request: Request, response_buffer: &'a mut Vec<u8> )
                        -> Response<'a> {
    let mut stream = match TcpStream::connect( request.node.to_string() ) {
        Ok( s ) => s,
        Err( _ ) => {
            let error_string = format!( "could not connect to {}",
                                         request.node.to_string() );
            response_buffer.extend_from_slice( error_string.as_bytes() );
            return Response {
                status: 1,
                message: &response_buffer[ .. error_string.len() ],
            };
        },
    };
    let request_bytes: &[u8] = request.request_string.as_bytes();

    let _ = stream.write( request_bytes );
    stream.shutdown( std::net::Shutdown::Write ).unwrap();

    let _ = stream.read_to_end( response_buffer );

    Response {
        status: response_buffer[ 0 ],
        message: &response_buffer[ 1.. ],
    }
}

fn perform_request_with_retry<'a>( request_string: &String,
                                   primary_node_option: Option< &Node >,
                                   backup_nodes: Vec< Node >,
                                   response_buffer: &'a mut Vec< u8 > )
                                   -> Option< Response<'a> > {
    let mut random_backup_nodes = backup_nodes.clone();
    let random_backup_nodes_slice = random_backup_nodes.as_mut_slice();
    rand::thread_rng().shuffle( random_backup_nodes_slice );

    let mut nodes: Vec< Node > = match primary_node_option {
        Some( primary_node ) => vec![ primary_node.clone() ],
        None => Vec::new(),
    };
    nodes.extend_from_slice( random_backup_nodes_slice );

    for node in nodes.iter() {
        let request = Request {
            node: node,
            request_string: request_string.clone(),
        };
        let temp_response_buffer = &mut Vec::new();
        let temp_response = perform_request( request, temp_response_buffer );
        if temp_response.status == 0 {
            response_buffer.clear();
            response_buffer.extend_from_slice( temp_response.message );
            return Some( Response {
                status: 0,
                message: &response_buffer[ 0.. ],
            } );
        }
    }
    None
}

fn request_length( nodes: Vec< Node >, file_name: &String ) -> u16 {
    let response_buffer = &mut Vec::new();
    let response_option = perform_request_with_retry(
        &format!( "{}:(LENGTH)", file_name ), None, nodes, response_buffer );

    match response_option {
        Some( response ) => {
            str::from_utf8( response.message ).unwrap().parse::<u16>().unwrap()
        },
        None => panic!( "Could not retrieve file length for {}", file_name ),
    }
}

fn request_whole_file<'a>( nodes: Vec< Node >, file_name: &String,
                           response_buffer: &'a mut Vec<u8> )
                           -> &'a [u8] {
    let response_option = perform_request_with_retry(
        &format!( "{}:(READ)", file_name ), None, nodes, response_buffer );

    match response_option {
        Some( response ) => {
            response.message
        },
        None => panic!( "Could not retrieve file contents for {}", file_name ),
    }
}

fn request_file_chunk<'a>( primary_node: &Node, backup_nodes: Vec< Node >,
                     file_name: &String, start_offset: u16, end_offset: u16,
                     response_buffer: &'a mut Vec<u8> ) -> &'a [u8] {
    let response_option = perform_request_with_retry(
        &format!( "{}:(READ,{},{})", file_name, start_offset, end_offset ),
        Some( primary_node ), backup_nodes, response_buffer );

    match response_option {
        Some( response ) => {
            response.message
        },
        None => panic!( "Could not retrieve file contents between {} and {} for {}",
                         start_offset, end_offset, file_name ),
    }
}

// split the file into even chunks based on how many nodes there are,
// then request the file chunks from the nodes
fn request_file_distributed( nodes: Vec< Node >, file_name: &String,
                             file_length: u16, file_contents: &mut Vec< u8 > ) {
    let num_nodes = nodes.len() as u16;
    let chunk_size: u16 = std::cmp::max( file_length / num_nodes, 1 );
    let mut start_offset = 0;
    let mut end_offset = 0;
    let mut count = 0;
    for node in nodes.clone() {
        end_offset += chunk_size;
        if end_offset > file_length {
            break;
        }
        if count == num_nodes - 1 {
            end_offset = file_length;
        }
        let response_buffer = &mut Vec::new();
        let file_chunk = request_file_chunk( &node, nodes.clone(), file_name,
                                              start_offset, end_offset,
                                              response_buffer );
        file_contents.extend_from_slice( file_chunk );
        start_offset = end_offset;
        count += 1;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[ 1 ];

    // retrieve /file_store.yaml
    let root_nodes = retrieve_root_nodes();
    let response_buffer = &mut Vec::new();
    let file_store = request_whole_file( root_nodes.clone(),
                                         &"/file_store.yaml".to_string(),
                                         response_buffer );

    // extract the file store map
    let file_store_string = str::from_utf8( file_store ).unwrap();
    let file_store_yaml = YamlLoader::load_from_str( &file_store_string ).unwrap();
    let file_store_map = file_store_yaml[ 0 ].as_hash().unwrap();

    // find nodes that have this file
    let ( checksum, active_nodes ) =
        retrieve_active_nodes( file_name, file_store_map );

    // retrieve file length
    let file_length = request_length( active_nodes.clone(), file_name );

    // retrieve the file
    let file_contents = &mut Vec::new();
    request_file_distributed( active_nodes.clone(), file_name,
                              file_length, file_contents );

    // verify file checksum matches the one in file store
    if checksum != crc32::checksum_ieee( file_contents ) {
        panic!( "File does not match checksum from file_store.yaml" );
    }

    // output the file to stdout
    io::stdout().write( file_contents ).unwrap();
}
