use std::sync::{LazyLock, Mutex, atomic};
use std::time::Duration;

use axum::{http, routing};
use socketioxide::{
    SocketIo,
    extract::{AckSender, Data, SocketRef},
    handler::Value,
};
use tower_http::cors;

static IS_CONNECTED: atomic::AtomicBool = atomic::AtomicBool::new(false);

/// Current head look direction (yaw, pitch) in radians, updated directly by the Socket.IO thread.
static HEAD_LOOK: LazyLock<Mutex<(f32, f32)>> = LazyLock::new(|| Mutex::new((0.0, 0.0)));

pub fn get_head_look() -> (f32, f32) {
    *HEAD_LOOK.lock().unwrap()
}

#[derive(serde::Deserialize)]
struct LookData {
    yaw: f32,
    pitch: f32,
}
static SERVER_S: Mutex<Option<crossbeam_channel::Sender<ServerMessage>>> = Mutex::new(None);
static SERVER_R: Mutex<Option<crossbeam_channel::Receiver<ServerMessage>>> = Mutex::new(None);
static CLIENT_S: Mutex<Option<crossbeam_channel::Sender<ClientMessage>>> = Mutex::new(None);
static CLIENT_R: Mutex<Option<crossbeam_channel::Receiver<ClientMessage>>> = Mutex::new(None);

pub enum ClientMessage {
    Frame { number: u64, swapchain_id: u64, jpeg_b64: String },
}

#[derive(serde::Serialize)]
struct FramePayload<'a> {
    number: u64,
    swapchain_id: u64,
    jpeg_b64: &'a str,
}

pub enum ServerMessage {
    Ping,
}

pub fn send_frame(number: u64, swapchain_id: u64, jpeg_b64: String) {
    let lock = CLIENT_S.lock().unwrap();
    let Some(client_s) = lock.as_ref() else {
        return;
    };

    if let Err(err) = client_s.try_send(ClientMessage::Frame { number, swapchain_id, jpeg_b64 }) {
        match err {
            crossbeam_channel::TrySendError::Full(_) => {}
            crossbeam_channel::TrySendError::Disconnected(_) => {
                panic!("channel is not supposed to be disconnected")
            }
        }
    }
}

pub fn process_server_message() -> Option<()> {
    let lock = SERVER_R.lock().unwrap();
    let server_r = lock.as_ref()?;

    match server_r.try_recv() {
        Ok(val) => {
            match val {
                ServerMessage::Ping => {
                    log::info!("received ping");
                }
            }

            Some(())
        }
        Err(err) => match err {
            crossbeam_channel::TryRecvError::Empty => None,
            crossbeam_channel::TryRecvError::Disconnected => {
                panic!("channel is not supposed to be disconnected")
            }
        },
    }
}

pub fn start() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let (server_s, server_r) = crossbeam_channel::bounded(25);
    SERVER_S.lock().unwrap().replace(server_s);
    SERVER_R.lock().unwrap().replace(server_r);
    let (client_s, client_r) = crossbeam_channel::bounded(25);
    CLIENT_S.lock().unwrap().replace(client_s);
    CLIENT_R.lock().unwrap().replace(client_r);

    std::thread::spawn(move || {
        rt.block_on(async move {
            let (socketio_layer, io) = SocketIo::new_layer();

            io.ns("/", async |socket: SocketRef| {
                if IS_CONNECTED
                    .compare_exchange(
                        false,
                        true,
                        atomic::Ordering::Acquire,
                        atomic::Ordering::Relaxed,
                    )
                    .is_err()
                {
                    log::error!("A client is already connected");
                    let _ = socket.disconnect();
                    return;
                }

                log::info!("Socket.IO connected {} {}", socket.ns(), socket.id);

                socket.on(
                    "message",
                    async |socket: SocketRef, Data::<String>(data)| {
                        let server_s = SERVER_S
                            .lock()
                            .unwrap()
                            .as_ref()
                            .expect("server_s channel is not set")
                            .clone();
                        log::info!("Received event: {:?}", data);
                        socket.emit("message-back", &data).ok();
                        if let Err(err) = server_s.send(ServerMessage::Ping) {
                            log::error!("{}", err);
                        }
                    },
                );

                socket.on(
                    "message-with-ack",
                    async |Data::<Value>(data), ack: AckSender| {
                        log::info!("Received event: {:?}", data);
                        ack.send(&data).ok();
                    },
                );

                socket.on("look", async |Data::<LookData>(data)| {
                    *HEAD_LOOK.lock().unwrap() = (data.yaw, data.pitch);
                });

                std::thread::spawn(move || {
                    while socket.connected() {
                        std::thread::sleep(Duration::from_millis(1));
                        if let Some(client_r) = CLIENT_R.lock().unwrap().as_ref() {
                            let res = match client_r.try_recv() {
                                Ok(msg) => match msg {
                                    ClientMessage::Frame { number, swapchain_id, jpeg_b64 } =>
                                        socket.emit("frame", &FramePayload {
                                            number,
                                            swapchain_id,
                                            jpeg_b64: &jpeg_b64,
                                        }),
                                },
                                Err(err) => match err {
                                    crossbeam_channel::TryRecvError::Empty => Ok(()),
                                    crossbeam_channel::TryRecvError::Disconnected => {
                                        panic!("channel is not supposed to be disconnected")
                                    }
                                },
                            };

                            if let Err(err) = res {
                                log::error!("error sending message: {err}");
                            }
                        }
                    }
                    log::info!("exit");
                    IS_CONNECTED.store(false, atomic::Ordering::Release);
                });
            });

            let cors = cors::CorsLayer::new()
                .allow_methods([http::Method::GET, http::Method::POST])
                .allow_origin(cors::Any);

            let app = axum::Router::new()
                .route(
                    "/",
                    routing::get(|| async { "Hello! Please use the web client to connect." }),
                )
                .layer(socketio_layer)
                .layer(cors);

            log::info!("Starting server");

            let listener = tokio::net::TcpListener::bind("0.0.0.0:3050").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        })
    });
}
