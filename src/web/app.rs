use std::{
    borrow::Cow,
    net::{Ipv4Addr, SocketAddr},
    ops::ControlFlow,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use axum_login::{
    login_required,
    tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use futures::{SinkExt, StreamExt};
use opencv::{
    core::Size,
    imgcodecs::{imdecode, IMREAD_COLOR},
    videoio::{VideoWriter, VideoWriterTrait},
};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use time::{Duration, OffsetDateTime};
use tokio::{
    net::TcpListener,
    signal,
    sync::watch,
    task::{AbortHandle, JoinHandle},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tower_http::services::ServeDir;
use tower_sessions::cookie::Key;
use tower_sessions_sqlx_store::SqliteStore;

// Allows to extract the IP of connecting user
use axum::{
    extract::{
        connect_info::ConnectInfo,
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
};
use axum::{
    extract::{ws::CloseFrame, State},
    Router,
};

use crate::{
    users::{AuthSession, Backend},
    web::{auth, protected},
    Model, Video,
};

const SQLITE_URL: &str = "sqlite://data.db";
const VIDEO_PATH: &str = "./videos/";

struct AppState {
    images_tx: watch::Sender<Message>,
    video_path: PathBuf,
}

pub struct App {
    pub db: SqlitePool,
    pub listener: TcpListener,
    pub video_path: PathBuf,
}

impl App {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let sqlite_connect_options =
            SqliteConnectOptions::from_str(SQLITE_URL)?.create_if_missing(true);

        let db = SqlitePool::connect_with(sqlite_connect_options).await?;

        sqlx::migrate!().run(&db).await?;

        let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3000));

        let listener = tokio::net::TcpListener::bind(addr).await?;

        let video_path_relative = PathBuf::from(VIDEO_PATH);
        let video_path = video_path_relative.canonicalize()?;

        Ok(Self {
            db,
            listener,
            video_path,
        })
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Session layer.
        //
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        let session_store = SqliteStore::new(self.db.clone());
        session_store.migrate().await?;

        let deletion_task = tokio::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
        );

        // Generate a cryptographic key to sign the session cookie.
        let key = Key::generate();

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::days(1)))
            .with_signed(key);

        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(self.db);
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        let vite_build_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web/dist/");

        // TODO: Use better default message
        let tx = watch::Sender::new(Message::Text("Hello, world!".to_string()));

        let state = Arc::new(AppState {
            images_tx: tx,
            video_path: self.video_path,
        });

        let main_router = Router::new()
            .route("/api/ws", axum::routing::any(ws_handler))
            .with_state(state);

        // TODO: Order of merge matters here, make sure the correct routes are protected and that fallback works as intended.
        let app = protected::router()
            .fallback_service(ServeDir::new(vite_build_dir).append_index_html_on_directories(true))
            .route_layer(login_required!(Backend, login_url = "/api/login"))
            .merge(main_router)
            .merge(auth::router())
            .layer(auth_layer);

        // Ensure we use a shutdown signal to abort the deletion task.
        axum::serve(
            self.listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        .await?;

        deletion_task.await??;

        Ok(())
    }
}

// ? Maybe move all functions below into App impl block, then use `self` for db pool

async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
    let ctrl_c = async {
        signal::ctrl_c().await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to install Ctrl+C handler");
        });
    };

    #[cfg(unix)]
    let terminate = async {
        let signal_result = signal::unix::signal(signal::unix::SignalKind::terminate());
        match signal_result {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => tracing::warn!(error = %e, "Failed to install Unix SIGTERM signal handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => { deletion_task_abort_handle.abort() },
        () = terminate => { deletion_task_abort_handle.abort() },
    }
}

// All WebSocket handling code based on axum example code
// https://github.com/tokio-rs/axum/blob/ffeb4f9407043dc6575a59f565e1ddec6cce227b/examples/websockets/src/main.rs

/// The handler for the HTTP request (this gets called when the HTTP request lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    println!("{addr} connected to ws_handler.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state, auth_session))
}

// TODO: Use tracing instead of print in this function
/// Actual websocket statemachine (one will be spawned per connection)
#[allow(clippy::too_many_lines)]
async fn handle_socket(
    socket: WebSocket,
    who: SocketAddr,
    state: State<Arc<AppState>>,
    auth_session: AuthSession,
) {
    println!("{who} connected to handle_socket.");

    // TODO: Update camera in DB to be online

    let mut images_rx_rec = state.images_tx.subscribe();

    let tracker = TaskTracker::new();
    let recording_token = CancellationToken::new();
    let recording_token_clone = recording_token.clone();
    let video_path = state.video_path.clone();

    // TODO: Init watch message may be consumed by any of the tasks, find way to avoid being off by one.
    //  Always ignoring the first message in every task is maybe not the best solution.

    // ? Maybe use spawn_blocking here, be aware .abort() is not available on blocking tasks
    // TODO: Handle stopping recording properly
    // TODO: Inform client/db if recording fails
    // TODO: Find out which is better, ingesting encoded or decoded images
    // TODO: Don't spawn record task for every websocket connection, only some are cameras
    let mut recording_task: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
        tracker.spawn(async move {
            let now = Video::DEFAULT.start_time();
            let formatted_now = now.format(Video::DEFAULT.file_name_format)?;
            let file_pathbuf = video_path.join(format!("{formatted_now}.avi"));

            // TODO: Lookup camera_id from DB
            // TODO: Use proper user customizable file path, ability to pass tempdir for tests would be nice
            let mut video = Video {
                video_id: Video::DEFAULT.video_id,
                camera_id: Some(2),
                file_path: file_pathbuf.to_string_lossy().to_string(),
                start_time: now,
                end_time: Video::DEFAULT.end_time,
                file_size: None,
            };

            #[allow(clippy::unwrap_used)]
            video.create_using_self(&auth_session.backend.db).await?;

            // TODO: Don't hardcode these
            let video_fourcc = VideoWriter::fourcc('m', 'p', '4', 'v')?;
            let video_size = Size::new(800, 600);
            let mut video_writer =
                VideoWriter::new_def(&video.file_path, video_fourcc, 12.5, video_size)?;

            let mut total_bytes = 0;

            let mut first_received = false;
            // TODO: Adding a sleep might be a good idea?
            loop {
                // TODO: this might not be the best way of doing this
                let message = (*images_rx_rec.borrow_and_update()).clone();
                let message_data_vec = message.into_data();
                let message_data_vec_slice = message_data_vec.as_slice();
                let decoded_image = imdecode(&message_data_vec_slice, IMREAD_COLOR)?;

                if first_received {
                    println!("Recording image from {who}...");
                    // TODO: Handle error here
                    // ? Does calling this function too often/quickly risk a crash? Use a buffer/batch?
                    video_writer.write(&decoded_image)?;
                    total_bytes += message_data_vec_slice.len();
                }

                tokio::select! {
                    c = images_rx_rec.changed() => {
                        if c.is_err() {
                            break;
                        }
                    },
                    () = recording_token.cancelled() => {
                        break;
                    }
                }

                first_received = true;
            }

            video.end_time = Some(OffsetDateTime::now_utc());
            video.file_size = Some(total_bytes.try_into()?);

            video.update_using_self(&auth_session.backend.db).await?;
            println!("Recording finished for {who}...");

            Ok(())
        });

    tracker.close();

    let mut images_rx = state.images_tx.subscribe();

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let mut send_task = tokio::spawn(async move {
        let mut first_received = false;
        // TODO: Adding a sleep might be a good idea?
        loop {
            // TODO: this might not be the best way of doing this
            let message = (*images_rx.borrow_and_update()).clone();

            if first_received {
                println!("Sending message to {who}...");
                // TODO: Don't send images to cameras. Maybe use a room system?
                // TODO: Handle error here
                let _ = sender.send(message).await;
            }

            if images_rx.changed().await.is_err() {
                break;
            }

            first_received = true;
        }

        println!("Sending close to {who}...");

        if let Err(e) = sender
            .send(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: Cow::from("Goodbye"),
            })))
            .await
        {
            println!("Could not send Close due to {e}, probably it is ok?");
        }
    });

    // This second task will receive messages from client and print them on server console
    // TODO: Reduce amount of cloning in this function
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            let message = process_message(msg.clone(), who);

            // print message and break if instructed to do so
            if message.is_break() {
                break;
            }

            // TODO: Make sure msg is Binary
            let _ = state.images_tx.send(msg);
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(()) => println!("send_task finished for {who}"),
                Err(a) => println!("Error sending messages {a:?}")
            }
            recv_task.abort();
            recording_token_clone.cancel();
            tracker.wait().await;
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(()) => println!("recv_task finished for {who}"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
            recording_token_clone.cancel();
            tracker.wait().await;
        },
        rv_c = (&mut recording_task) => {
            match rv_c {
                Ok(_) => println!("recording_task finished for {who}"),
                Err(c) => println!("Error recording images {c:?}")
            }
            // ? Maybe do something if recording fails e.g. send a message to the client/DB
        }
    }

    // TODO: Update camera in DB to be offline

    // returning from the handler closes the websocket connection
    println!("Websocket context {who} destroyed");
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent str: {t:?}");
        }
        Message::Binary(d) => {
            println!(">>> {} sent {} bytes", who, d.len());
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> {who} sent pong with {v:?}");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {who} sent ping with {v:?}");
        }
    }
    ControlFlow::Continue(())
}
