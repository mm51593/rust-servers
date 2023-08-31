use std::{
    io::{self, Read, Write},
    net::{TcpListener}, os::fd::AsRawFd, collections::HashMap,
};

use epoll::{Events, Event};

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

// Multiplexed server
pub fn main() {
    // instantiate non-blocking listener
    let listener = TcpListener::bind("localhost:3000").unwrap();
    listener.set_nonblocking(true).unwrap();

    // create epoll
    let epoll = epoll::create(false).unwrap();

    // create event
    let event = Event::new(Events::EPOLLIN, listener.as_raw_fd() as _);
    epoll::ctl(epoll, epoll::ControlOptions::EPOLL_CTL_ADD, listener.as_raw_fd(), event).unwrap();

    // store connections
    let mut connections = HashMap::new();

    // listen to incoming connections
    loop {
        let mut events = [Event::new(Events::empty(), 0); 1024];

        let num_events = epoll::wait(epoll, 0, &mut events).unwrap();
        let mut completed = Vec::new();

        'next: for event in &events[..num_events] {
            let fd = event.data as i32;

            // is the listener ready?
            if fd == listener.as_raw_fd() {
                // try accepting a connection
                match listener.accept() {
                    Ok((connection, _)) => {
                        connection.set_nonblocking(true).unwrap();
                        let fd = connection.as_raw_fd();

                        let event = Event::new(Events::EPOLLIN | Events::EPOLLOUT, fd as _);
                        epoll::ctl(epoll, epoll::ControlOptions::EPOLL_CTL_ADD, fd, event).unwrap();

                        let state = ConnectionState::Read { request: [0u8; 1024], read: 0 };

                        connections.insert(fd, (connection,state));
                    },
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {},
                    Err(e) => panic!("{e}"),
                }

                continue 'next;
            }

            // otherwise, a connection must be ready
            let (connection, state) = connections.get_mut(&fd).unwrap();

            if let ConnectionState::Read { request, read } = state {
                loop {
                    // read some number of bytes into buffer
                    match connection.read(&mut request[*read..]) {
                        Ok(0) => {
                            // client disconnected
                            println!("client disconnected unexpectedly");
                            completed.push(fd);
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

                *state = ConnectionState::Write { response: response.as_bytes(), written: 0 };
            }

            if let ConnectionState::Write { response, written } = state {
                loop {
                    // write some number of bytes
                    match connection.write(&response[*written..]) {
                        Ok(0) => {
                            // client disconnected
                            println!("client disconnected unexpectedly");
                            completed.push(fd);
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

                *state = ConnectionState::Flush;
            }

            if let ConnectionState::Flush = state {
                match connection.flush() {
                    Ok(_) => {
                        completed.push(fd);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue 'next;
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        }

        for fd in completed {
            let (connection, _state) = connections.remove(&fd).unwrap();
            // unregister from epoll
            drop(connection);
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
