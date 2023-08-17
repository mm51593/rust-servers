use std::{net::{TcpListener, TcpStream}, io::{self, Read, Write}};

// Multi-threaded server
pub fn main() {
    // instantiate listener
    let listener = TcpListener::bind("localhost:3000").unwrap();

    // listen to incoming connections
    loop {
        let (connection, _) = listener.accept().unwrap();

        // handle them in separate threads
        std::thread::spawn(|| {
            if let Err(e) = handle_connection(connection) {
                println!("failed to handle connectino: {e}")
            }
        });
    }
}

fn handle_connection(mut connection: TcpStream) -> io::Result<()> {
    // initialise buffer
    let mut request = [0u8; 1024];
    // bytes read
    let mut read = 0;

    loop {
        // read some number of bytes into buffer
        let num_bytes = connection.read(&mut request[read..])?;

        // client disconnected
        if num_bytes == 0 {
            println!("client disconnected unexpectedly");
            return Ok(());
        }
        
        // increment number of read bytes
        read += num_bytes;

        // check for the end of the request
        if request.get(read - 4..read) == Some(b"\r\n\r\n") {
            break;
        }
    }

    // print request to console
    let request = String::from_utf8_lossy(&request[..read]);
    println!("{request}");

    // construct response
    // Hello World! in HTTP
    let response = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Length: 12\r\n",
        "Connection: close\r\n\r\n",
        "Hello world!"
    );

    let mut written = 0;

    loop {
        // write some number of bytes
        let num_bytes = connection.write(response[written..].as_bytes())?;

        // client disconnected
        if num_bytes == 0 {
            println!("client disconnected unexpectedly");
            return Ok(());
        }

        written += num_bytes;

        // check if all bytes have been written
        if written == response.len() {
            break;
        }
    }

    // flush
    connection.flush()
}

#[cfg(test)]
mod test {
    use super::main;

    #[test]
    fn basics() {
        main();
    }
}