use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;

struct Client {
    name: String,
    stream: TcpStream,
    address: SocketAddr,
}

fn handle_client_connection(mut client: Client, clients: Arc<Mutex<Vec<Client>>>) {
    let client_address = client.address;
    let mut buffer_reader = BufReader::new(client.stream.try_clone().unwrap());
    loop {
        let mut message = String::new();
        match buffer_reader.read_line(&mut message) {
            Ok(0)=>{
                println!("Connection closed by client {}", client.name);
                break;
            }
            Ok(_) => {
                let formated_message = format!("{}: {}", client.name, message);
                println!("Received: {}", formated_message);
                // Send this message to other clients (without sender)
                let clients_lock = clients.lock().unwrap();

                for other in clients_lock.iter() {
                    //we don't want to send this back to sender
                    if other.address != client_address {
                        if let Ok(mut other_stream) = other.stream.try_clone() {
                            let _ = other_stream.write_all(formated_message.as_bytes());
                        }
                    }
                }

            }
            Err(e) => {
                eprintln!("Error reading from client {}: {}", client.name, e);
                break;
            }
        }
    }
    //after loop ends we want to remove client from list of clients
    let mut clients_lock = clients.lock().unwrap();
    clients_lock.retain(|c| c.address != client_address);
}

fn run_server() -> std::io::Result<()> {
    let port = 321321;
    let tcp_listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    let udp_socket = UdpSocket::bind(format!("127.0.0.1:{}", port))?;
    udp_socket.set_nonblocking(true)?;
    let clients = Arc::new(Mutex::new(Vec::<Client>::new()));

    // Udp handling
    {
        let udp_socket = udp_socket.try_clone()?;
        let clients = Arc::clone(&clients);
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                match udp_socket.recv_from(&mut buffer) {
                    Ok((size, src)) => {
                        let message = String::from_utf8_lossy(&buffer[..size]);
                        println!("Received UDP: {}", message.trim());
                        //send to every other client but sender
                        let client_lock = clients.lock().unwrap();
                        for client in client_lock.iter() {
                            if client.address != src {
                                let _ = udp_socket.send_to(message.as_bytes(), client.address);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data - let's wait for a moment
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e)=>{
                        eprintln!("Error receiving UDP: {}", e)
                    }
                }
            }
        })
    }
    println!("Server listening on port {}", port);
    for stream in tcp_listener.incoming() {
        match stream {
            Ok(stream) => {
                let address = stream.peer_addr()?;
                let mut reader = BufReader::new(stream.try_clone()?);
                let mut client_name = String::new();
                reader.read_line(&mut client_name)?;
                let client_name = client_name.trim().to_string();
                println!("Client connected: {} / {}", client_name, address);
                let client = Client {
                    name: client_name.clone(),
                    stream: stream.try_clone()?,
                    address,
                };
                clients.lock().unwrap().push(client);
                let clients_clone = Arc::clone(&clients);
                thread::spawn(move || {
                    handle_client_connection(Client { name: client_name, stream, address}, clients_clone);
                });

            }
            Err(e) => {
                eprintln!("Error reading TCP stream: {}", e);
            }
        }
    }
    Ok(())
}