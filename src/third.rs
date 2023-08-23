use std::{
    io::{self, Read, Write},
    net::{TcpListener},
};

enum ConnectionState {
    Read {
        request: [u8; 1024],
        read: usize,
    },
    Write {
        response: &'static [u8],
        written: usize,
    },
    Flush,
}

// Non-blocking server
pub fn main() {
    // instantiate non-blocking listener
    let listener = TcpListener::bind("localhost:3000").unwrap();
    listener.set_nonblocking(true).unwrap();

    // store connections
    let mut connections = Vec::new();

    // listen to incoming connections
    loop {
        match listener.accept() {
            Ok((connection, _)) => {
                // make conenction nonblocking
                connection.set_nonblocking(true).unwrap();

                // initialise state
                let state = ConnectionState::Read {
                    request: [0u8; 1024],
                    read: 0,
                };

                connections.push((connection, state));
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        };

        // store completed connections
        let mut completed = Vec::new();

        'next: for (i, (connection, state)) in connections.iter_mut().enumerate() {
            if let ConnectionState::Read { request, read } = state {
                loop {
                    // read some number of bytes into buffer
                    match connection.read(&mut request[*read..]) {
                        Ok(0) => {
                            // client disconnected
                            println!("client disconnected unexpectedly");
                            completed.push(i);
                            continue 'next;
                        }
                        Ok(n) => {
                            // increment number of read bytes
                            *read += n;
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // not ready, move onto the next connection
                            continue 'next;
                        }
                        Err(e) => panic!("{e}"),
                    }

                    // check for the end of the request
                    if request.get(*read - 4..*read) == Some(b"\r\n\r\n") {
                        break;
                    }
                }

                // print request to console
                let request = String::from_utf8_lossy(&request[..*read]);
                println!("{request}");

                // construct response
                // Hello World! in HTTP
                let response = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Content-Length: 12\r\n",
                    "Connection: close\r\n\r\n",
                    "Hello world!"
                );

                // move into write state
                *state = ConnectionState::Write {
                    response: response.as_bytes(),
                    written: 0,
                };
            }

            if let ConnectionState::Write { response, written } = state {
                loop {
                    // write some number of bytes
                    match connection.write(&response[*written..]) {
                        Ok(0) => {
                            // client disconnected
                            println!("client disconnected unexpectedly");
                            completed.push(i);
                            continue 'next;
                        }
                        Ok(n) => {
                            *written += n;
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // not ready, move onto the next connection
                            continue 'next;
                        }
                        Err(e) => panic!("{e}"),
                    }

                    // check if all bytes have been written
                    if *written == response.len() {
                        break;
                    }
                }

                // successfully wrote the response, move into flush state
                *state = ConnectionState::Flush;
            }

            if let ConnectionState::Flush = state {
                match connection.flush() {
                    Ok(_) => {
                        completed.push(i);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue 'next;
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        }

        // remove completed connections
        for i in completed.into_iter().rev() {
            connections.remove(i);
        }
    }
}

#[cfg(test)]
mod test {
    use super::main;

    #[test]
    fn basics() {
        main();
    }
}
