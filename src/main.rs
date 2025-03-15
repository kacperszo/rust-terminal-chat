use std::io;

mod server;
mod client;

fn main() {
    let port = 8080;
    println!("Enter S to start server or C to start client:");
    let mut mode = String::new();
    io::stdin().read_line(&mut mode).ok();
    match mode.trim() {
        "S" => {
            server::run_server(port).expect("Error while starting server");
        },
        "C" => {
            client::run_client(port).expect("Error while starting client");
        },
        _ => {
            println!("Invalid mode");
        }
    }

}
