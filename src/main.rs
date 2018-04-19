extern crate tokio;
extern crate regex;

use regex::Regex;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use tokio::prelude::*;
use tokio::io;
use tokio::net::TcpListener;

// operation result in the form: Ok( message, info ), Err( why )
type OperResult = Result< ( String, String ), String >;

fn main() {
    let address = "127.0.0.1:2345".parse().unwrap();
    let listener = TcpListener::bind( &address )
        .expect( "cannot bind TCP listener" );

    println!( "Running file server on {}", address );

    // handle incoming connections
    let server = listener.incoming()
        .map_err( | e | eprintln!( "accept failed = {:?}", e ) )
        .for_each( | socket | {
            let peer_addr = socket.peer_addr().unwrap();

            // split socket into reader and writer
            let ( reader, writer ) = socket.split();

            let buf: Vec<u8> = Vec::new();

            let handle_conn = io::read_to_end( reader, buf )
                .and_then( move | ( _, buf ) | {
                    /* OPERATION FUNCTIONS */
                    
                    // write file contents (or error) into writer socket
                    // and log info
                    let send_and_log = | oper_result: OperResult | {
                        match oper_result {
                            Err( why ) => {
                                eprintln!( "{}: {}", peer_addr, why );
                                io::write_all( writer, format!( "{}\n", why ) )
                            },
                            Ok( ( result, info ) ) => {
                                println!( "{}: {}",
                                           peer_addr, info );
                                io::write_all( writer, result )
                            },
                        }  
                    };

                    // open file and read from it
                    let file_read = | file_name, start_offset, end_offset | {
                        let path = Path::new( file_name );
                        match File::open( &file_name ) {
                            Err( why ) => Err( format!(
                                "couldn't open {}: {}", path.display(),
                                why.description() ) ),
                            Ok( mut file ) => {
                                // read file contents into string
                                let mut file_contents = String::new();
                                let file_read_oper =
                                    match file.read_to_string( &mut file_contents ) {
                                        Err( why ) => Err( format!(
                                            "couldn't read {}: {}", path.display(),
                                            why.description() ) ),
                                        Ok( _ ) => Ok( () ),
                                    };
                                // handle start offset
                                file_contents = file_contents
                                    .chars()
                                    .skip( start_offset as usize )
                                    .collect();
                                // handle end offset
                                if end_offset != -1 {
                                    file_contents = file_contents
                                        .chars()
                                        .take( end_offset as usize -
                                               start_offset as usize )
                                        .collect();
                                }
                                match file_read_oper {
                                    Err( why ) => Err( why ),
                                    Ok( _ ) => Ok( ( file_contents, format!(
                                        "sending file contents of {}, \
                                         start offset {}, end offset {}",
                                        file_name, start_offset, end_offset ) ) ),
                                }
                            },
                        }
                    };
                    
                    let file_length = | file_name | {
                        match std::fs::metadata( file_name ) {
                            Err( why ) => Err( format!(
                                "couldn't get info for {}: {}",
                                file_name, why.description() ) ),
                            Ok( m ) => Ok( ( format!( "{}", m.len() ),
                                             format!( "sending file length of {}: {}",
                                                       file_name, m.len() ) ) )
                        }
                    };

                    /* ERROR MESSAGES */
                    
                    let format_err = Err( format!(
                        "invalid request, \
                         correct format: filename:(action,arg1,arg2,...)" ) );

                    let too_many_args_err = | num_args, action | {
                        Err( format!(
                            "too many arguments ({}) provided for the action: {}",
                            num_args, action ) )
                    };

                    let num_arg_err = | n, why | {
                        Err( format!(
                            "could not convert \"{}\" to unsigned integer: {}",
                            n, why ) )
                    };

                    let unrecognized_action_err = | action | {
                        Err( format!( "unrecognized action: {}", action ) )
                    };

                    /* REQUEST PARSING */

                    let request_contents = std::str::from_utf8( &buf[ .. ] ).unwrap();
                    
                    // requests must be in the form: filename:(action,arg1,arg2,...)
                    let request_re = Regex::new( r"^([^:]+):\(([^()]+)\)$" ).unwrap();
                    let captures = match request_re.captures( request_contents ) {
                        Some( m ) => m,
                        None => {
                            return send_and_log( format_err );
                        },
                    };
                    
                    let file_name = match captures.get( 1 ) {
                        Some( m ) => m.as_str(),
                        None => {
                            return send_and_log( format_err );
                        },
                    };

                    let actions_vec = match captures.get( 2 ) {
                        Some( m ) => {
                            let mut action_str = m.as_str();
                            action_str.split( "," ).collect::< Vec< &str > >()
                        },
                        None => {
                            return send_and_log( format_err );
                        },
                    };

                    if actions_vec.len() == 0 {
                        return send_and_log( format_err );
                    }

                    let action = actions_vec[ 0 ];
                    let action_args = &actions_vec[ 1 .. ];

                    /* EXECUTE AND RETURN REQUEST */

                    if action == "READ" {
                        let mut start_offset = 0;
                        let mut end_offset = -1;
                        if action_args.len() >= 1 {
                            let start_offset_str = action_args[ 0 ];
                            start_offset =
                                match start_offset_str.parse::< i32 >() {
                                    Ok( n ) => n,
                                    Err( why ) => {
                                        return send_and_log( num_arg_err(
                                            start_offset_str, why ) );
                                    },
                                };
                        }
                        if action_args.len() >= 2 {
                            let end_offset_str = action_args[ 1 ];
                            end_offset =
                                match end_offset_str.parse::< i32 >() {
                                    Ok( n ) => n,
                                    Err( why ) => {
                                        return send_and_log( num_arg_err(
                                            end_offset_str, why ) );
                                    },
                                };                                
                        }
                        if action_args.len() > 2 {
                            return send_and_log(
                                too_many_args_err( action_args.len(), action ) );
                        }
                        return send_and_log( file_read( file_name,
                                                        start_offset,
                                                        end_offset ) );
                    } else if action == "LENGTH" {
                        if action_args.len() == 0 {
                            return send_and_log( file_length( file_name ) );
                        } else {
                            return send_and_log(
                                too_many_args_err( action_args.len(), action ) );
                        }
                    } else {
                        return send_and_log( unrecognized_action_err( action ) );
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
