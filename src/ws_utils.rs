use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::{TcpSocket, TcpStream};
use tokio_tungstenite::tungstenite::error::{Error, UrlError};
use tokio_tungstenite::tungstenite::handshake::client::{Request, Response};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::{client_async_tls_with_config, MaybeTlsStream, WebSocketStream};

// taken from tokio-tungstenite source code
#[allow(dead_code)]
#[inline]
fn domain(request: &Request) -> Result<String, WsError> {
    #[allow(clippy::option_if_let_else)]
    match request.uri().host() {
        // rustls expects IPv6 addresses without the surrounding [] brackets
        // #[cfg(feature = "__rustls-tls")]
        // Some(d) if d.starts_with('[') && d.ends_with(']') => Ok(d[1..d.len() - 1].to_string()),
        Some(d) => Ok(d.to_string()),
        None => Err(Error::Url(UrlError::NoHostName)),
    }
}

/// Modified version of `tokio-tungstenite::connect()` that allows specifying the client port.
///
/// <https://github.com/snapview/tokio-tungstenite/blob/015e00d9ccb447161ab69f18946d501c71d0f689/src/connect.rs#L73-L98>
pub async fn same_port_connect(
    request: Request,
    client_port: u16,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    let domain = domain(&request)?;
    let port = request
        .uri()
        .port_u16()
        .or_else(|| match request.uri().scheme_str() {
            Some("wss") => Some(443),
            Some("ws") => Some(80),
            _ => None,
        })
        .ok_or(Error::Url(UrlError::UnsupportedUrlScheme))?;

    let addr = format!("{domain}:{port}");
    let Ok(socket_addr) = addr.parse() else {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Failed to parse address",
        )));
    };

    let socket = TcpSocket::new_v4()?;

    // ? Might be better to not call this on Windows https://docs.rs/tokio/latest/tokio/net/struct.TcpSocket.html
    socket.set_reuseaddr(true)?;
    let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), client_port);
    socket.bind(client_addr)?;

    let stream = socket.connect(socket_addr).await.map_err(Error::Io)?;
    // let socket = TcpStream::connect(addr).await.map_err(tokio_tungstenite::tungstenite::error::Error::Io)?;

    client_async_tls_with_config(request, stream, None, None).await
}
