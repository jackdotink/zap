mod api;
mod builder;
mod client;
mod ir;
mod serdes;
mod server;
mod types;

fn main() {
    let items = match api::exec(std::fs::read("test.luau").unwrap().as_slice()) {
        Ok(ty) => ty,
        Err(e) => {
            eprintln!("Error: {e}");
            return;
        }
    };

    std::fs::write(
        "client.luau",
        client::client(
            client::Client {
                apicheck: serdes::ApiCheck::Full,
                location: String::new(),
            },
            &items,
        )
        .unwrap(),
    )
    .unwrap();

    std::fs::write(
        "server.luau",
        server::server(
            server::Server {
                apicheck: serdes::ApiCheck::Full,
                location: String::new(),
            },
            &items,
        )
        .unwrap(),
    )
    .unwrap();
}
