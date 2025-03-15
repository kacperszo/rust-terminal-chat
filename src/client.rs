use std::fmt::format;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpStream, UdpSocket, SocketAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, Shutdown};
use std::process::exit;
use socket2::{Socket, Domain, Type, Protocol};
use std::thread;

fn create_udp_socket_with_reuse(port: i32) -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    let address: SocketAddrV4 = format!("0.0.0.0:{}", port).parse().unwrap();
    socket.bind(&address.into())?;
    Ok(socket.into())
}

pub(crate) fn run_client(port: i32) -> std::io::Result<()> {
    let server_address = format!("127.0.0.1:{}", port);
    let mut tcp_stream = TcpStream::connect(server_address.clone())?;
    let local_address = tcp_stream.local_addr()?;
    println!("Connected to {} from {}", server_address, local_address);

    println!("Enter your name:");
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    name = name.trim().to_string();
    tcp_stream.write_all(format!("{}\n", name).as_bytes())?;


    //UDP connection for multicast
    let udp_socket = UdpSocket::bind(format!("0.0.0.0:{}", local_address.port())).unwrap_or_else(|_| {
        UdpSocket::bind("0.0.0.0:0").expect("Failed to bind UDP socket")//when we cant connect using the same port why use ephemeric port
    });

    udp_socket.connect(server_address)?;
    // multicast config
    let multicast_address = format!("239.0.0.1:{}", port);

    let multicast_socket = create_udp_socket_with_reuse(port)?;
    multicast_socket.set_broadcast(true)?;
    multicast_socket.join_multicast_v4(&Ipv4Addr::new(239,0,0,1), &Ipv4Addr::UNSPECIFIED).ok();
    multicast_socket.set_multicast_loop_v4(true)?;
    //receiving TCP thread
    {
        let tcp_stream = tcp_stream.try_clone()?;
        thread::spawn(move || {
           let reader = BufReader::new(tcp_stream);
            for line in reader.lines() {
                match line {
                    Ok(message) => println!("{}", message),
                    Err(_) => { break; }
                }
            }
        });
    }
    //receiving UDP thread
    {
        let udp_socket = udp_socket.try_clone()?;
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                match udp_socket.recv(&mut buffer) {
                    Ok(size) => {
                        let message = String::from_utf8_lossy(&buffer[..size]);
                        println!("UDP: {}", message);
                    }
                    Err(_) => {}
                }
            }
        });
    }
    //Multicast receiving thread
    {
        let multicast_socket = multicast_socket.try_clone()?;
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                match multicast_socket.recv(&mut buffer) {
                    Ok(size) => {
                        let message = String::from_utf8_lossy(&buffer[..size]);
                        println!("Multicast UDP: {}", message);
                    }
                    Err(_) => {}
                }
            }
        });
    }
    //Main loop
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let input = line?;
        if input.starts_with("U ") {
            // UDP sending
            let msg = input.strip_prefix("U ").unwrap();
            udp_socket.send(msg.as_bytes())?;
        } else if input.starts_with("M ") {
            // Multicast sending
            let msg = input.strip_prefix("M ").unwrap();
            multicast_socket.send_to(msg.as_bytes(), multicast_address.clone())?;
        } else if input.eq("!quit") {
            println!("Quitting...");
            tcp_stream.shutdown(Shutdown::Both).ok();
            multicast_socket.leave_multicast_v4(&Ipv4Addr::new(239,0,0,1), &Ipv4Addr::UNSPECIFIED).ok();
            exit(0);
        }
        else {
            // TCP sending
            tcp_stream.write_all((input + "\n").as_bytes())?;
        }
    }


    Ok(())
}