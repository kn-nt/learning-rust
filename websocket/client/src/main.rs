use tungstenite::{connect, Message};

fn main() {
    let (mut socket, response) = connect("ws://localhost:3000/wst").expect("Can't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (header, _value) in response.headers() {
        println!("* {header}");
    }

    socket.send(Message::Text("Hello WebSocket".into())).unwrap();
    loop {
        let msg = socket.read();
        if let Ok(Message::Binary(ref bin_data)) = msg {
            let msg: nx::WSRequest = bincode::deserialize(&bin_data).expect("Failed to deserialize");
            println!("Received message: {:?}", msg);
        }
        // println!("Received message: {:?}", msg);
    }
    // socket.close(None);
}