use axum::extract::ws::{Message, WebSocket};
use axum::extract::{path, Query};
use axum::{extract::WebSocketUpgrade, response::IntoResponse, routing::get, Router};
use std::collections::HashMap;
use std::str::from_utf8;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use std::{env, fs};
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() {

    // println!("Sleeping");
    // thread::sleep(Duration::from_millis(3000));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/wsb", get(ws_handler_binary))
        .route("/wst", get(ws_handler_test));
    // .with_state(COMPLETE_HASH_MAP.get());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    println!("p listening on {}", listener.local_addr().unwrap());
    // axum::serve(
    //     listener,
    //     app.into_make_service_with_connect_info::<SocketAddr>(),
    // )
    // .await
    // .unwrap();
    axum::serve(listener, app).await.unwrap();
}


#[axum::debug_handler]
async fn ws_handler(
    // Query(params): Query<nx_hoster::Params>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    println!("pre handler");
    ws.on_upgrade(|ws: WebSocket| async {
        println!("handler");
        // stream_data(ws, params).await;
        stream_data(ws).await;
    })
}

async fn stream_data(mut ws: WebSocket) {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}



#[axum::debug_handler]
async fn ws_handler_binary(
    // Query(params): Query<nx_hoster::Params>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    println!("pre handler");
    ws.on_upgrade(|ws: WebSocket| async {
        println!("handler");
        // stream_data(ws, params).await;
        crate::stream_data_binary(ws).await;
    })
}

async fn stream_data_binary(mut ws: WebSocket) {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}


#[axum::debug_handler]
async fn ws_handler_test(
    // Query(params): Query<nx_hoster::Params>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    println!("pre handler");
    ws.on_upgrade(|ws: WebSocket| async {
        println!("handler");
        // stream_data(ws, params).await;
        crate::stream_data_test(ws).await;
    })
}

async fn stream_data_test(mut ws: WebSocket) {
    loop {
        println!("HIT");
        let data = "testing";
        let data = nx::WSRequest{path: "testingasdfsdfsdfdf".parse().unwrap() };
        let encoded: Vec<u8> = bincode::serialize(&data).unwrap();
        // ws.send(Message::Text(encoded.iter().map(|x| x.to_string()).collect())).await.unwrap();
        ws.send(Message::Binary(encoded)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
