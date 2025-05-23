use std::{
    borrow::Cow,
    net::{Ipv4Addr, SocketAddr},
    ops::ControlFlow,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use axum_embed::ServeEmbed;
use axum_login::{
    login_required,
    tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use axum_server::tls_rustls::RustlsConfig;
use futures_util::{SinkExt, StreamExt};
use opencv::{
    core::Size,
    imgcodecs::{imdecode, IMREAD_COLOR},
    videoio::{VideoWriter, VideoWriterTrait},
};
use rust_embed::RustEmbed;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use time::{Duration, OffsetDateTime};
use tokio::{
    net::TcpListener,
    signal,
    sync::{watch, Mutex},
    task::{AbortHandle, JoinHandle},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
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
use tracing::{debug, error, info, warn};

use crate::{
    users::{AuthSession, Backend},
    web::{auth, protected, CameraListChange, CameraMessage},
    ApiChannelMessage, Camera, CameraPermissionView, CameraSetting, CameraSettingNoMeta, Model,
    User, Video,
};

use super::{ImageContainer, MdnsChannelMessage};

// TODO: Maybe use `std::future::pending::<()>();` instead of sleeping forever

// TODO: Change default admin and guest hashes, remember to search and update where they're hardcoded
const SQLITE_PROD_URL: &str = "sqlite://oko.db";
const SQLITE_DEV_URL: &str = "sqlite://data.db";
const VIDEO_PATH: &str = "./videos/";
const DEFAULT_ADMIN_USERNAME: &str = "admin";
const DEFAULT_ADMIN_PASS_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw";
const EXPIRED_SESSION_DELETION_INTERVAL: tokio::time::Duration =
    tokio::time::Duration::from_secs(60);
const SESSION_DURATION: Duration = Duration::days(1);
const CAMERA_INDICATOR_TEXT: &str = "camera";
const CAMERA_ANY_PORT_INDICATOR_TEXT: &str = "camera_any_port";
const EMPTY_TASK_SLEEP_DURATION: tokio::time::Duration = tokio::time::Duration::from_millis(100);

#[derive(RustEmbed, Clone)]
#[folder = "static/"]
struct EmbeddedAssets;

pub struct AppState {
    pub images_tx: watch::Sender<ImageContainer>,
    pub video_path: PathBuf,
    pub api_channel: watch::Sender<ApiChannelMessage>,
    pub mdns_channel: watch::Sender<MdnsChannelMessage>,
    pub shutdown_token: CancellationToken,
    pub oko_private_socket_addr: Option<SocketAddr>,
    pub db_pool: SqlitePool,
}

pub struct App {
    pub db: SqlitePool,
    pub http_listener: TcpListener,
    pub https_addr: Option<SocketAddr>,
    pub video_path: PathBuf,
    pub oko_private_socket_addr: Option<SocketAddr>,
}

impl App {
    #[allow(clippy::cognitive_complexity)]
    #[allow(clippy::similar_names)]
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let sqlite_connect_options = if cfg!(debug_assertions) {
            SqliteConnectOptions::from_str(SQLITE_DEV_URL)?.create_if_missing(true)
        } else {
            let Ok(password) = std::env::var("OKO_DB_PASSWORD") else {
                error!("No password provided for database. Please provide a password using the OKO_DB_PASSWORD environment variable.");
                return Err("No password provided for database. Please provide a password using the OKO_DB_PASSWORD environment variable.".into());
            };

            SqliteConnectOptions::from_str(SQLITE_PROD_URL)?
                .create_if_missing(true)
                .pragma("key", password)
                .pragma("cipher_page_size", "1024")
                .pragma("kdf_iter", "64000")
                .pragma("cipher_hmac_algorithm", "HMAC_SHA1")
                .pragma("cipher_kdf_algorithm", "PBKDF2_HMAC_SHA1")
        };

        let db = SqlitePool::connect_with(sqlite_connect_options).await?;

        sqlx::migrate!().run(&db).await?;

        let http_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3080));
        let http_listener = tokio::net::TcpListener::bind(http_addr).await?;

        let https_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3443));

        info!("Listening on: {} and {}", http_addr, https_addr);

        let video_path_relative = PathBuf::from(VIDEO_PATH);

        if !video_path_relative.exists() {
            std::fs::create_dir_all(&video_path_relative)?;
        }

        let video_path = video_path_relative.canonicalize()?;

        debug!("Video path: {:?}", video_path);

        // Private IPv4 e.g. 192.168.x.x with port for passing to camera
        let oko_private_socket_addr = SocketAddr::from((
            local_ip_address::local_ip()?,
            http_listener.local_addr()?.port(),
        ));

        Ok(Self {
            db,
            http_listener,
            https_addr: Some(https_addr),
            video_path,
            oko_private_socket_addr: Some(oko_private_socket_addr),
        })
    }

    #[allow(clippy::similar_names)]
    #[allow(clippy::too_many_lines)] // TODO: Refactor
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // ? Maybe make this optional just in case
        let admin_exists = User::get_using_username(&self.db, DEFAULT_ADMIN_USERNAME)
            .await
            .is_ok();
        if !admin_exists {
            let mut admin = User {
                user_id: User::DEFAULT.user_id,
                username: "admin".to_string(),
                password_hash: DEFAULT_ADMIN_PASS_HASH.to_owned(),
                created_at: User::DEFAULT.created_at(),
            };

            admin.create_using_self(&self.db).await?;
        }

        // Session layer.
        //
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        let session_store = SqliteStore::new(self.db.clone());
        session_store.migrate().await?;

        let deletion_task = tokio::spawn(
            session_store
                .clone()
                .continuously_delete_expired(EXPIRED_SESSION_DELETION_INTERVAL),
        );

        // Generate a cryptographic key to sign the session cookie.
        let key = Key::generate();

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(SESSION_DURATION))
            .with_signed(key);

        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(self.db.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        let embedded_assets_service = ServeEmbed::<EmbeddedAssets>::new();

        let tx = watch::Sender::new(ImageContainer {
            camera_id: -1,
            timestamp: -1,
            image_bytes: vec![],
        });

        let api_channel = watch::Sender::new(ApiChannelMessage::Initial);

        let mdns_channel = watch::Sender::new(MdnsChannelMessage::Initial);

        let shutdown_token = CancellationToken::new();

        let app_state = Arc::new(AppState {
            images_tx: tx,
            video_path: self.video_path,
            api_channel: api_channel.clone(),
            mdns_channel: mdns_channel.clone(),
            shutdown_token: shutdown_token.clone(),
            oko_private_socket_addr: self.oko_private_socket_addr,
            db_pool: self.db,
        });

        let mdns_task = tokio::spawn(async move {
            let Ok(mdns_discovery) = mdns::discover::interface(
                "_http._tcp.local",
                tokio::time::Duration::from_secs(5),
                Ipv4Addr::UNSPECIFIED,
            ) else {
                error!("Failed to create mDNS discovery");
                return;
            };
            let mdns_stream = mdns_discovery.listen();
            futures_util::pin_mut!(mdns_stream);

            while let Some(Ok(mdns_response)) = mdns_stream.next().await {
                let (Some(_host), Some(_addr)) =
                    (mdns_response.hostname(), mdns_response.socket_address())
                else {
                    continue;
                };

                // debug!("Discovered service using mDNS: {host} {addr}");

                mdns_channel.send_replace(MdnsChannelMessage::ServiceDiscovered { mdns_response });
            }
        });

        let main_router = Router::new()
            .route("/api/ws", axum::routing::any(ws_handler))
            .route("/api/guest_exists", axum::routing::get(guest_exists_route))
            .with_state(app_state.clone());

        // TODO: Order of merge matters here, make sure the correct routes are protected and that fallback works as intended.
        let app = protected::router(app_state)
            .fallback_service(embedded_assets_service)
            .route_layer(login_required!(Backend, login_url = "/api/login"))
            .merge(main_router)
            .merge(auth::router())
            .layer(auth_layer);

        let axum_rustls_handle = axum_server::Handle::new();

        let https_addr_clone = self.https_addr;
        let app_clone = app.clone();
        let axum_rustls_handle_clone = axum_rustls_handle.clone();

        let https_task = tokio::spawn(async move {
            let Some(https_addr) = https_addr_clone else {
                return Ok(());
            };

            let Ok(tls_config) =
                RustlsConfig::from_pem_file("./certs/oko.internal.crt", "./certs/oko.internal.key")
                    .await
                    .map_err(|e| {
                        error!("Failed to load TLS config, no HTTPS server will be created: {e:?}");
                        e
                    })
            else {
                return Ok(());
            };

            let server = axum_server::bind_rustls(https_addr, tls_config);

            // For some reason, enabling connect protocol causes issues when connecting
            // server.http_builder().http2().enable_connect_protocol();

            server
                .handle(axum_rustls_handle_clone)
                .serve(
                    app_clone
                        .clone()
                        .into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
        });

        // Ensure we use a shutdown signal to abort the deletion task.
        axum::serve(
            self.http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal(
            deletion_task.abort_handle(),
            mdns_task.abort_handle(),
            axum_rustls_handle,
            shutdown_token,
        ))
        .await?;

        let (mdns_task_result, deletion_task, https_task) =
            tokio::join!(mdns_task, deletion_task, https_task);

        mdns_task_result?;
        deletion_task??;
        https_task??;

        Ok(())
    }
}

pub async fn guest_exists_route(state: State<Arc<AppState>>) -> impl IntoResponse {
    let guest_user_result = User::get_using_username(&state.db_pool, "guest").await;
    if guest_user_result.is_err() {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    http::StatusCode::OK.into_response()
}

// ? Maybe move all functions below into App impl block, then use `self` for db pool

async fn shutdown_signal(
    deletion_task_abort_handle: AbortHandle,
    mdns_task_abort_handle: AbortHandle,
    axum_rustls_handle: axum_server::Handle,
    shutdown_token: CancellationToken,
) {
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
        () = ctrl_c => {
            deletion_task_abort_handle.abort();
            shutdown_token.cancel();
            mdns_task_abort_handle.abort();
            axum_rustls_handle.shutdown();
        },
        () = terminate => {
            deletion_task_abort_handle.abort();
            shutdown_token.cancel();
            mdns_task_abort_handle.abort();
            axum_rustls_handle.shutdown();
        },
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
    info!("{addr} connected to ws_handler.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state, Arc::new(auth_session)))
}

// ! Camera restart does not guarantee new recording, frames will keep going to the same video unless socket times out?
// TODO: Find out if ECONNRESET after a while of no messages only affects vite dev server or if it is a general issue
// ? Using tick await for empty tasks might not be the best idea
/// Actual websocket statemachine (one will be spawned per connection)
#[allow(clippy::too_many_lines)]
async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    state: State<Arc<AppState>>,
    auth_session: Arc<AuthSession>,
) {
    info!("{who} connected to handle_socket.");

    // TODO: Update camera in DB to be online

    let mut images_rx_rec = state.images_tx.subscribe();

    let tracker = TaskTracker::new();
    let recording_token = CancellationToken::new();
    let recording_token_clone = recording_token.clone();
    let video_path = state.video_path.clone();

    // TODO: Init watch message may be consumed by any of the tasks, find way to avoid being off by one.
    //  Always ignoring the first message in every task is maybe not the best solution.

    let mut is_camera = false;
    let mut camera_any_port = false;
    let mut camera_id: i64 = -1;

    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg.clone(), who).is_break() {
                return;
            }

            match msg {
                Message::Text(msg_txt) => {
                    if msg_txt == CAMERA_INDICATOR_TEXT {
                        info!("{who} is a camera...");
                        is_camera = true;
                    } else if msg_txt == CAMERA_ANY_PORT_INDICATOR_TEXT {
                        // TODO: Maybe find a better way to handle this
                        info!("{who} is a camera (any port)...");
                        is_camera = true;
                        camera_any_port = true;
                    } else {
                        info!("{who} is not camera...");
                    }
                }
                _ => {
                    debug!("Ignoring first message from {who}...");
                }
            }
        } else {
            warn!("client {who} abruptly disconnected");
            return;
        }
    }

    let mut initial_camera_settings = None;
    let cameras: Arc<Mutex<Vec<CameraPermissionView>>> = Arc::new(Mutex::new(Vec::new()));
    let mut user_id = None;

    if is_camera {
        // TODO: Maybe find a better way to handle this
        if camera_any_port {
            let Ok(db_camera) =
                Camera::get_using_ip(&state.db_pool, who.ip().to_string() + ":*").await
            else {
                // TODO: Inform client/db if camera not found (both web user and ws connection), also find better way to exit here?
                error!("Camera (any port) not found in DB, aborting...");
                return;
            };

            camera_id = db_camera.camera_id;
        } else {
            let Ok(db_camera) = Camera::get_using_ip(&state.db_pool, who.to_string()).await else {
                // TODO: Inform client/db if camera not found (both web user and ws connection), also find better way to exit here?
                error!("Camera not found in DB, aborting...");
                return;
            };

            camera_id = db_camera.camera_id;
        }

        let Ok(camera_settings) = CameraSetting::get_for_camera(&state.db_pool, camera_id).await
        else {
            error!("Error getting initial camera settings for camera {camera_id}, aborting...");
            return;
        };

        initial_camera_settings = Some(camera_settings);
    } else {
        // TODO: Return errors to user
        let Some(ref user) = auth_session.user else {
            error!("User not found in auth session...");
            return;
        };

        user_id = Some(user.user_id);

        let Ok(i_cameras) = Camera::list_accessible_to_user(&state.db_pool, user.user_id).await
        else {
            error!("Error listing cameras for user...");
            return;
        };

        *cameras.lock().await = i_cameras;
    }

    let initial_camera_settings_clone = initial_camera_settings.clone();

    // ? Maybe use spawn_blocking here, be aware .abort() is not available on blocking tasks
    // ? Maybe assume is camera if IP belongs to camera in DB
    // TODO: Handle stopping recording properly
    // TODO: Inform client/db if recording fails
    // TODO: Find out which is better, ingesting encoded or decoded images
    // ! Camera restart does not guarantee new recording, frames will keep going to the same video unless socket times out?
    let mut recording_task: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
        if is_camera {
            let state_clone = state.clone();
            // TODO: Check if errors are returned properly here, had some issues with the ? operator being silent
            tracker.spawn(async move {
                let now = Video::DEFAULT.start_time();
                let formatted_now = now.format(Video::DEFAULT.file_name_format)?;
                let file_pathbuf = video_path.join(format!("{formatted_now}.avi"));

                // TODO: Lookup camera_id from DB
                // TODO: Use proper user customizable file path, ability to pass tempdir for tests would be nice
                let mut video = Video {
                    video_id: Video::DEFAULT.video_id,
                    camera_id: Some(camera_id),
                    file_path: file_pathbuf.to_string_lossy().to_string(),
                    start_time: now,
                    end_time: Video::DEFAULT.end_time,
                    file_size: None,
                };

                // ? Maybe don't create video until first frame (or maybe doing this is actually a good approach)?
                video.create_using_self(&state_clone.db_pool).await?;

                let (frame_width, frame_height, framerate) = match initial_camera_settings_clone {
                    #[allow(clippy::match_same_arms)] // readability
                    Some(settings) => {
                        let (frame_width, frame_height) = match settings.resolution.as_str() {
                            "SVGA" => (800, 600),
                            "VGA" => (640, 480),
                            _ => (800, 600),
                        };

                        (frame_width, frame_height, settings.framerate)
                    }
                    None => (800, 600, 12),
                };

                // TODO: Don't hardcode these
                let video_fourcc = VideoWriter::fourcc('m', 'p', '4', 'v')?;
                let video_size = Size::new(frame_width, frame_height);
                // TODO: Investigate why video is too fast
                #[allow(clippy::cast_precision_loss)] // the precision loss is acceptable
                let mut video_writer = VideoWriter::new_def(
                    &video.file_path,
                    video_fourcc,
                    framerate as f64,
                    video_size,
                )?;

                let mut total_bytes = 0;

                let mut first_received = false;
                // TODO: Adding a sleep might be a good idea?
                // ! Camera restart does not guarantee new recording, frames will keep going to the same video unless socket times out?
                // TODO: Handle if user changes resolution/framerate during recording (this is likely to happen)
                loop {
                    // TODO: this might not be the best way of doing this
                    let message = (*images_rx_rec.borrow_and_update()).clone();

                    if first_received {
                        debug!("Recording image from {who}...");

                        // ? Parsing JSON for every image just to see if the camera matches is wasteful, is there a better way?
                        if message.camera_id == camera_id {
                            let message_data_vec = message.image_bytes;
                            // let message_data_vec = message.into_data();
                            let message_data_vec_slice = message_data_vec.as_slice();
                            let decoded_image = imdecode(&message_data_vec_slice, IMREAD_COLOR)?;

                            // TODO: Handle error here
                            // ? Does calling this function too often/quickly risk a crash? Use a buffer/batch?
                            video_writer.write(&decoded_image)?;
                            total_bytes += message_data_vec_slice.len();
                        }
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

                video.update_using_self(&state_clone.db_pool).await?;
                info!("Recording finished for {who}...");

                Ok(())
            })
        } else {
            tracker.spawn(async move {
                let mut interval = tokio::time::interval(EMPTY_TASK_SLEEP_DURATION);
                loop {
                    tokio::select! {
                        _ = interval.tick() => {},
                        () = recording_token.cancelled() => {
                            break;
                        }
                    }
                }

                Ok(())
            })
        };

    tracker.close();

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (sender, mut receiver) = socket.split();
    // TODO: Investigate performance of Tokio Mutex, there could be better options
    let sender_mutex = Arc::new(Mutex::new(sender));

    // Spawn a task that will push several messages to the client (does not matter what client does)
    #[allow(clippy::if_not_else)]
    let mut send_task: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
        if !is_camera {
            let cameras_clone = cameras.clone();
            let mut images_rx = state.images_tx.subscribe();
            let sender_mutex_clone = sender_mutex.clone();
            // TODO: Proper error handling
            tokio::spawn(async move {
                let mut first_received = false;
                // TODO: Adding a sleep might be a good idea?
                loop {
                    // TODO: this might not be the best way of doing this
                    let message = (*images_rx.borrow_and_update()).clone();

                    if first_received {
                        debug!("Sending message to {who}...");

                        // TODO: locking this mutex in such a hot path is a horrible idea, absolutely find a better way to do this
                        if cameras_clone
                            .lock()
                            .await
                            .iter()
                            .any(|c| c.camera_id == message.camera_id)
                        {
                            // TODO: look into bincode (fastest?) / rmp-serde (wide support) / flatbuffers (partial deserialization)
                            let message_json = serde_json::to_string(&message)?;
                            let message_json_msg = Message::Text(message_json.clone());

                            // TODO: Handle error here
                            sender_mutex_clone
                                .lock()
                                .await
                                .send(message_json_msg)
                                .await?;
                        }
                    }

                    if images_rx.changed().await.is_err() {
                        break;
                    }

                    first_received = true;
                }

                info!("Sending close to {who}...");

                if let Err(e) = sender_mutex_clone
                    .lock()
                    .await
                    .send(Message::Close(Some(CloseFrame {
                        code: axum::extract::ws::close_code::NORMAL,
                        reason: Cow::from("Goodbye"),
                    })))
                    .await
                {
                    warn!("Could not send Close due to {e}, probably it is ok?");
                }

                Ok(())
            })
        } else {
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(EMPTY_TASK_SLEEP_DURATION);
                loop {
                    interval.tick().await;
                }
            })
        };

    // This second task will receive messages from client and print them on server console
    // TODO: Reduce amount of cloning in this function
    let recv_state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            process_message(msg.clone(), who);

            match msg.clone() {
                Message::Binary(_) => {
                    if !is_camera {
                        continue;
                    }

                    let img_container = ImageContainer {
                        camera_id,
                        timestamp: OffsetDateTime::now_utc().unix_timestamp(),
                        image_bytes: msg.into_data(),
                    };

                    let _ = recv_state_clone.images_tx.send(img_container);
                }
                Message::Close(_) => break,
                _ => (),
            }
        }
    });

    let api_channel = state.api_channel.clone();
    let sender_mutex_clone = sender_mutex.clone();
    let mut first_received = false;

    let mut api_listener_task: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
        if is_camera {
            tokio::spawn(async move {
                if let Some(some_camera_settings) = initial_camera_settings {
                    let some_initial_camera_settings = CameraSettingNoMeta {
                        flashlight_enabled: some_camera_settings.flashlight_enabled,
                        resolution: some_camera_settings.resolution,
                        framerate: some_camera_settings.framerate,
                    };

                    let initial_camera_setting_message =
                        CameraMessage::SettingChanged(some_initial_camera_settings);

                    if let Err(e) = sender_mutex_clone
                        .lock()
                        .await
                        .send(Message::Text(serde_json::to_string(
                            &initial_camera_setting_message,
                        )?))
                        .await
                    {
                        error!("Error sending initial camera settings to {who}: {e:?}");
                    }
                }

                let mut api_channel_rx = api_channel.subscribe();
                loop {
                    let api_msg = (*api_channel_rx.borrow_and_update()).clone();

                    if first_received {
                        #[allow(clippy::single_match)] // will change in the future
                        match api_msg {
                            ApiChannelMessage::CameraAction {
                                camera_id: message_camera_id,
                                message,
                            } => {
                                if message_camera_id == camera_id {
                                    info!("API channel message received for api_camera_id {message_camera_id} for {who}...");

                                    if let Err(e) = sender_mutex_clone
                                        .lock()
                                        .await
                                        .send(Message::Text(serde_json::to_string(&message)?))
                                        .await
                                    {
                                        error!(
                                            "Error sending API WebSocket message to {who}: {e:?}"
                                        );
                                    }
                                }
                            }
                            ApiChannelMessage::CameraListChanged(change) => match change {
                                CameraListChange::Removed {
                                    camera_id: camera_id_removed,
                                } => {
                                    if camera_id_removed == camera_id {
                                        info!("Closing WebSocket for {who} because camera was removed from DB");
                                        return Err("Camera removed from DB".into());
                                    }
                                }
                                _ => (),
                            },
                            ApiChannelMessage::Initial => (),
                        }
                    }

                    if api_channel_rx.changed().await.is_err() {
                        break;
                    }

                    first_received = true;
                }

                Ok(())
            })
        } else {
            tokio::spawn(async move {
                let mut api_channel_rx = api_channel.subscribe();
                loop {
                    let api_msg = (*api_channel_rx.borrow_and_update()).clone();

                    if first_received {
                        #[allow(clippy::single_match)] // will change in the future
                        match api_msg {
                            // TODO: every user task performing this is wasteful, global camera list mutex shared with api would be better
                            ApiChannelMessage::CameraListChanged(change) => {
                                info!("API channel message received for camera list changed for {who}...");

                                let Some(user_id_some) = user_id else {
                                    error!("Camera list changed but user was not found in auth_session. How is this even possible?");
                                    break;
                                };

                                if let Err(e) = sender_mutex_clone
                                    .lock()
                                    .await
                                    .send(Message::Text(serde_json::to_string(&change)?))
                                    .await
                                {
                                    error!("Error sending API WebSocket message to {who}: {e:?}");
                                }

                                let Ok(new_cameras) =
                                    Camera::list_accessible_to_user(&state.db_pool, user_id_some)
                                        .await
                                else {
                                    // TODO: prevent potential endless loop here from the DB call always failing
                                    error!("Error listing new cameras for {who}...");
                                    continue;
                                };

                                *cameras.lock().await = new_cameras;
                            }
                            _ => (),
                        }
                    }

                    if api_channel_rx.changed().await.is_err() {
                        break;
                    }

                    first_received = true;
                }

                Ok(())
            })
        };

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(_) => info!("send_task finished for {who}"),
                Err(a) => error!("Error sending messages {a:?}")
            }
            recv_task.abort();
            api_listener_task.abort();
            recording_token_clone.cancel();
            tracker.wait().await;
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(()) => info!("recv_task finished for {who}"),
                Err(b) => error!("Error receiving messages {b:?}")
            }
            send_task.abort();
            api_listener_task.abort();
            recording_token_clone.cancel();
            tracker.wait().await;
        },
        rv_c = (&mut recording_task) => {
            match rv_c {
                Ok(_) => info!("recording_task finished for {who}"),
                Err(c) => error!("Error recording images {c:?}")
            }
            // ? Maybe do something if recording fails e.g. send a message to the client/DB
        },
        rv_d = (&mut api_listener_task) => {
            match rv_d {
                Ok(_) => info!("api_listener_task finished for {who}"),
                Err(d) => error!("Error listening to API channel {d:?}")
            }
            send_task.abort();
            recv_task.abort();
            recording_token_clone.cancel();
            tracker.wait().await;
            // ? Maybe do something if api channel fails e.g. send a message to the client/DB
        }
    }

    // TODO: Update camera in DB to be offline

    // returning from the handler closes the websocket connection
    info!("Websocket context {who} destroyed");
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
#[allow(clippy::cognitive_complexity)]
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            if t.len() > 100 {
                let short_t = &t[..100];
                debug!(">>> {who} sent str: {short_t}...");
            } else {
                debug!(">>> {who} sent str: {t:?}");
            }
        }
        Message::Binary(d) => {
            debug!(">>> {} sent {} bytes", who, d.len());
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                info!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                error!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            debug!(">>> {who} sent pong with {v:?}");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            debug!(">>> {who} sent ping with {v:?}");
        }
    }
    ControlFlow::Continue(())
}
