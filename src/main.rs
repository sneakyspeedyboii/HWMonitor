use rand::Rng;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async};
use std::{net::SocketAddr, fmt::format, sync::Mutex, thread::Thread};
use tungstenite::{Message};
use futures_util::{StreamExt, SinkExt};
use axum::{routing::{get}, response::{Html, IntoResponse}, Router};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref TEMPERATURE: Mutex<String> = Mutex::new(String::new());
}

#[tokio::main]
async fn main() {
    tokio::spawn(api_server());
    let addr = "127.0.0.1:7887";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    println!("Listening on: {}", addr);

    while let Ok((stream, sock)) = listener.accept().await {
        let peer = stream.peer_addr().unwrap();
        println!("New socket connection on: {}, Peer: {}", sock.to_string(), peer.to_string());
        tokio::spawn(handle_connection(peer, stream));
    }  
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) {
    println!("New thread spawned: Peer = {}", peer.to_string());
    let ws_stream = accept_async(stream).await.expect("Accept fail");

    let (mut ws_sender, mut ws_reciever) = ws_stream.split();

    while let Some(response) = ws_reciever.next().await {
        let message = response.expect("error unwrapping").to_string();
        println!("Resonse: {}", message);
        let mut state = TEMPERATURE.lock().unwrap();
        *state = String::from(message);
        println!("Mutex: {}", state);
    }
}

async fn api_server() {
    let app = Router::new()
        .route("/", get(root));

    let addr = SocketAddr::from(([127, 0, 0, 1], 7667));

    println!("Axum listining on: {}", addr.to_string());

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> impl IntoResponse {
    let data = format!("<div>{}<div>", TEMPERATURE.lock().unwrap());
    Html(data)
    
}

