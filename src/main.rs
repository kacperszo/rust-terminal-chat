mod server;
mod client;

fn main() {
    server::run_server().expect("Error while starting server");
}
