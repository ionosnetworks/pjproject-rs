use axum::{
    extract::{
        ws::{self, Message, WebSocket, WebSocketUpgrade},
        Query, TypedHeader,
    },
    headers,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use clap::{Arg, Command};
use futures::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use pjproject_rs as pj;
use serde::Deserialize;
use std::{collections::hash_map::Entry, ffi::CString, net::SocketAddr, sync::Arc};
use thiserror::Error;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

mod sip;

static PJSIP: OnceCell<sip::PjSip> = OnceCell::new();

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    PjError(#[from] pj::Error),
    #[error("{0}")]
    Validation(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = axum::http::StatusCode::from_u16(500).unwrap();
        (status, self.to_string()).into_response()
    }
}

#[tokio::main]
async fn main() {
    let env_filter = tracing_subscriber::filter::EnvFilter::builder()
        .with_default_directive(concat!(env!("CARGO_CRATE_NAME"), "=info").parse().unwrap())
        .with_env_var("RUST_LOG")
        .from_env_lossy()
        .add_directive("tower_http=debug".parse().unwrap())
        .add_directive("pjproject_rs=warn".parse().unwrap());

    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(env_filter)
            .finish(),
    )
    .expect("setting tracing default failed");

    tracing_log::LogTracer::init().expect("failed to init tracing log");

    let matches = Command::new("Sip Call")
        .arg(
            Arg::new("call-uri")
                .long("call-uri")
                .help("SIP URI to call")
                .takes_value(true)
                .value_name("URI")
                .required(true)
                .env("CALL_URI"),
        )
        .get_matches();

    let call_uri = CString::new(matches.get_one::<String>("call-uri").unwrap().as_str()).expect("Failed to parse call_uri");
    let (quit_tx, quit_rx) = flume::bounded(1);
    ctrlc::set_handler(move || {
        tracing::warn!("Received ctrl-c, stopping server...");
        let _ = quit_tx.send(());
    })
    .expect("Error setting Ctrl-C handler");

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(call_uri));

    // run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], 7000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            let _ = quit_rx.recv_async().await;
            if let Ok(pjsip) = sip::get_sip() {
                pjsip.calls.lock().drain().for_each(|(id, mut c)| {
                    tracing::info!("Found call with id: {id}");
                    if let Some(call) = c.take() {
                        tracing::info!(call_id = id, "Ending call..");
                        let (call_rx, call_media) = {
                            let call = call.lock();
                            (call.rx.clone(), call.media.clone())
                        };
                        if let Err(_) = call.lock().hang_up() {
                            tracing::error!(call_id = id, "Failed to hang up call");
                        }
                        if let Err(_) = call_rx.recv_timeout(std::time::Duration::from_secs(10)) {
                            tracing::error!("Expected to receive disconnect from call");
                        }

                        if let Err(_) = call_media.lock().destroy() {
                            tracing::error!(
                                call_id = id,
                                "Failed to send quit to call media thread"
                            )
                        };
                    }
                });
            }
        })
        .await
        .unwrap();
}

#[derive(Deserialize)]
struct WsParams {
    timescale: u32,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    Query(params): Query<WsParams>,
    Extension(call_uri): Extension<CString>,
) -> Result<impl IntoResponse, Error> {
    let f = || {
        if let Some(TypedHeader(user_agent)) = user_agent {
            tracing::debug!("`{}` connected", user_agent.as_str());
        }

        let sip = PJSIP.get_or_try_init(|| sip::init_sip::<&std::ffi::CStr>(None, None))?;
        let id = "camera-speaker-id".to_string();
        let create_call = match sip.calls.lock().entry(id.clone()) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(None);

                true
            }
        };

        if !create_call {
            return Err(Error::Validation("Busy".into()));
        }

        let call = match sip::Call::make_call(&id, params.timescale, call_uri) {
            Ok(call) => {
                sip.calls.lock().insert(id, Some(call.clone()));

                call
            }
            Err(err) => {
                sip.calls.lock().remove(&id);
                return Err(err);
            }
        };

        Ok::<_, Error>(call)
    };

    let call = match f() {
        Ok(c) => c,
        Err(err) => {
            tracing::error!("Failed to create call: {err}");
            return Err(err);
        }
    };

    Ok(ws.on_upgrade(|ws| handle_socket(ws, call)))
}

async fn handle_socket(socket: WebSocket, call: Arc<Mutex<sip::Call>>) {
    let (call_id, mut wav_strmr, call_media, call_rx) = {
        let call = call.lock();
        let wav_strmr = match call.wav_strmr.clone() {
            Some(c) => c,
            None => return,
        };

        (
            call.id.clone(),
            wav_strmr,
            call.media.clone(),
            call.rx.clone(),
        )
    };

    enum Msg {
        Ws(Result<axum::extract::ws::Message, axum::Error>),
        Sip(sip::Msg),
    }
    let (mut ws_tx, ws_rx) = socket.split();
    let mut rx = futures::stream::select(
        ws_rx.map(|m| Msg::Ws(m)),
        call_rx.clone().into_stream().map(|m| Msg::Sip(m)),
    );
    while let Some(msg) = rx.next().await {
        match msg {
            Msg::Sip(msg) => match msg {
                sip::Msg::InvState(state) => {
                    if matches!(state, pj::PjSipInvState::Disconnected) {
                        if let Err(_) = call_media.lock().destroy() {
                            tracing::error!(call_id = call_id, "Failed to destroy call media")
                        }

                        if let Err(err) = ws_tx.send(ws::Message::Close(None)).await {
                            tracing::error!(
                                call_id = call_id,
                                "Failed to send close to websocket client: {err}"
                            );
                        }

                        return;
                    }
                }
                sip::Msg::Media(res) => match res {
                    Ok(_) => {}
                    Err(err) => {
                        if let Err(_) = call_media.lock().destroy() {
                            tracing::error!(call_id = call_id, "Failed to destroy call media")
                        }

                        if let Err(err) = ws_tx
                            .send(ws::Message::Close(Some(ws::CloseFrame {
                                code: ws::close_code::ERROR,
                                reason: std::borrow::Cow::Owned(err.to_string()),
                            })))
                            .await
                        {
                            tracing::error!(
                                call_id = call_id,
                                "Failed to send close to websocket client: {err}"
                            );
                        }
                    }
                },
            },
            Msg::Ws(msg) => match msg {
                Ok(msg) => match msg {
                    Message::Binary(data) => {
                        if !wav_strmr.initialized() {
                            wav_strmr
                                .initialize(&data)
                                .expect("Failed to initialize wav_strmr");
                        } else {
                            wav_strmr.add_data(&data);
                        }
                    }
                    Message::Close(_) => {
                        tracing::warn!("client disconnected");
                        let _ = sip::get_sip().map(|pjsip| pjsip.calls.lock().remove(&call_id));
                        if let Err(_) = call.lock().hang_up() {
                            tracing::error!(call_id = call_id, "Failed to hang up call");
                        }
                        if let Err(_) = call_rx.recv_timeout(std::time::Duration::from_secs(10)) {
                            tracing::error!("Expected to receive disconnect from call");
                        }

                        if let Err(_) = call_media.lock().destroy() {
                            tracing::error!(
                                call_id = call_id,
                                "Failed to send quit to call media thread"
                            )
                        }

                        return;
                    }
                    _ => (),
                },
                Err(err) => {
                    tracing::error!(
                        call_id = call_id,
                        "Failed to receive: client disconnected: {err}"
                    );
                    return;
                }
            },
        }
    }
}
