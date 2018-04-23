extern crate tokio;
extern crate regex;

use regex::Regex;
use std::fs::File;
use std::path::Path;
use std::io::Seek;
use tokio::prelude::*;
use tokio::io;
use tokio::net::TcpListener;

// operation result in the form: Ok( message, info ), Err( why )
type OperResult = Result< ( Vec< u8 >, String ), String >;

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

            // requests must be in the form: filename:(action,arg1,arg2,...)
            let request_re = Regex::new( r"^([^:]+):\(([^()]+)\)$" ).unwrap();

            // split socket into reader and writer
            let ( reader, writer ) = socket.split();

            let buf: Vec<u8> = Vec::new();

            let handle_conn = io::read_to_end( reader, buf )
                .and_then( move | ( _, buf ) | {
                    /* ERROR MESSAGES */
                    
                    let format_err = Err( format!(
                        "invalid request, \
                         correct format: filename:(action,arg1,arg2,...)" ) );

                    let too_many_args_err = | num_args, action | {
                        Err( format!(
                            "too many arguments ({}) provided for the action: {}",
                            num_args, action ) )
                    };

                    let num_arg_err = | why, n | {
                        Err( format!(
                            "could not convert \"{}\" to unsigned integer: {}",
                            n, why ) )
                    };

                    let unrecognized_action_err = | action | {
                        Err( format!( "unrecognized action: {}", action ) )
                    };

                    let no_info_err = | why, file_name | {
                        Err( format!(
                            "could not retrieve file metadata for {}: {}",
                            file_name, why ) )
                    };

                    /* OPERATION FUNCTIONS */
                    
                    // write file contents (or error) into writer socket
                    // and log info
                    let send_and_log = | oper_result: OperResult | {
                        match oper_result {
                            Ok( ( result, info ) ) => {
                                println!( "{}: {}",
                                           peer_addr, info );
                                io::write_all( writer, result )
                            },
                            Err( why ) => {
                                eprintln!( "{}: {}", peer_addr, why );
                                io::write_all( writer,
                                               format!( "{}\n", why ).into_bytes() )
                            },
                        }  
                    };

                    macro_rules! unwrap_option {
                        ( $o:expr, $e:expr ) => {
                            {
                                match $o {
                                    Some( v ) => v,
                                    None => {
                                        return send_and_log( $e );
                                    },
                                }
                            }
                        };
                    }

                    macro_rules! unwrap_result {
                        ( $r:expr, $e:expr ) => {
                            {
                                match $r {
                                    Ok( v ) => v,
                                    Err( why ) => {
                                        return send_and_log(
                                            $e( why ) );
                                    },
                                }
                            }
                        };
                        ( $r:expr, $e:expr, $( $a:expr ),* ) => {
                            {
                                match $r {
                                    Ok( v ) => v,
                                    Err( why ) => {
                                        return send_and_log(
                                            $e( why, $( $a ),* ) );
                                    },
                                }
                            }
                        };
                    }

                    // open file and read from it
                    let file_read_func = | file_name, start_offset, end_offset | {
                        let path = Path::new( file_name );
                        let length: usize = end_offset as usize -
                            start_offset as usize;

                        match File::open( &file_name ) {
                            Ok( mut file ) => {
                                let mut file_buf = vec![ 0u8; length ];

                                file.seek( std::io::SeekFrom::Start(
                                    start_offset as u64 ) ).unwrap();
                                
                                let file_read_oper =
                                    match file.read_exact( &mut file_buf ) {
                                        Ok( _ ) => Ok( () ),
                                        Err( why ) => Err( format!(
                                            "could not read {}: {}", path.display(),
                                            why ) ),
                                    };

                                match file_read_oper {
                                    Ok( _ ) => Ok( ( file_buf, format!(
                                        "sending file contents of {}, \
                                         start offset {}, end offset {}",
                                        file_name, start_offset, end_offset ) ) ),
                                    Err( why ) => Err( why ),
                                }
                            },
                            Err( why ) => Err( format!(
                                "couldn't open {}: {}", path.display(),
                                why ) ),
                        }
                    };
                    
                    let file_length_func = | file_name | {
                        match std::fs::metadata( file_name ) {
                            Ok( m ) => Ok( ( format!( "{}", m.len() ).into_bytes(),
                                             format!( "sending file length of {}: {}",
                                                       file_name, m.len() ) ) ),
                            Err( why ) => no_info_err( why, file_name ),
                        }
                    };

                    /* REQUEST PARSING */

                    let request_contents = std::str::from_utf8( &buf[ .. ] ).unwrap();

                    let captures = unwrap_option!(
                        request_re.captures( request_contents ), format_err );
                    
                    let file_name = unwrap_option!(
                        captures.get( 1 ), format_err ).as_str();

                    let actions_vec = unwrap_option!(
                        captures.get( 2 ), format_err )
                        .as_str()
                        .split( "," )
                        .collect::< Vec< &str > >();

                    if actions_vec.len() == 0 {
                        return send_and_log( format_err );
                    }

                    let action = actions_vec[ 0 ];
                    let action_args = &actions_vec[ 1 .. ];

                    // we need file length for a couple of different things,
                    // so we go ahead and retrieve that now
                    let file_length = unwrap_result!(
                        std::fs::metadata( file_name ), no_info_err, file_name )
                        .len() as u32;

                    if action == "READ" {
                        let mut start_offset = 0;
                        let mut end_offset = file_length;
                        if action_args.len() >= 1 {
                            let start_offset_str = action_args[ 0 ];
                            start_offset = unwrap_result!(
                                start_offset_str.parse::< u32 >(), num_arg_err,
                                String::from( start_offset_str ) );
                            if start_offset >= file_length {
                                return send_and_log( Err( format!(
                                    "start offset ({}) must be: \
                                     less than file length ({})",
                                    start_offset, file_length ) ) );
                            }
                        }
                        if action_args.len() >= 2 {
                            let end_offset_str = action_args[ 1 ];
                            end_offset = unwrap_result!(
                                end_offset_str.parse::< u32 >(), num_arg_err,
                                String::from( end_offset_str ) );
                            if end_offset < 1 || end_offset > file_length ||
                                end_offset <= start_offset {
                                return send_and_log( Err( format!(
                                    "end offset ({}) must be: \
                                     greater than zero, \
                                     greater than start offset ({}), \
                                     less than or equal to file length ({})",
                                    end_offset, start_offset, file_length ) ) );
                            }
                        }
                        if action_args.len() > 2 {
                            return send_and_log(
                                too_many_args_err( action_args.len(), action ) );
                        }
                        return send_and_log( file_read_func( file_name,
                                                             start_offset,
                                                             end_offset ) );
                    } else if action == "LENGTH" {
                        if action_args.len() == 0 {
                            return send_and_log( file_length_func( file_name ) );
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
