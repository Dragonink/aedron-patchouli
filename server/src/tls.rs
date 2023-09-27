//! Provides TLS capabilities

use axum::extract::connect_info::Connected;
use hyper::server::{
	accept::Accept,
	conn::{AddrIncoming, AddrStream},
};
use std::{
	error::Error,
	fmt::{self, Display, Formatter},
	fs,
	future::Future,
	io::{self, IoSlice},
	net::SocketAddr,
	path::Path,
	pin::Pin,
	task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_native_tls::{
	native_tls::{self, Identity, Protocol},
	TlsAcceptor,
};

/// Reads the TLS identity from a certificate and a private key files
#[inline]
pub(crate) fn read_identity(certificate: &Path, key: &Path) -> io::Result<Identity> {
	Identity::from_pkcs8(&fs::read(certificate)?, &fs::read(key)?)
		.map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

/// TLS wrapper for [`AddrIncoming`]
pub(crate) struct TlsAddrIncoming {
	/// Wrapped [`AddrIncoming`]
	inner: AddrIncoming,
	/// TLS acceptor
	tls: TlsAcceptor,
}
impl TlsAddrIncoming {
	/// Constructs a new instance binding to the given socket address
	#[inline]
	pub(crate) fn bind(addr: &SocketAddr, identity: Identity) -> Result<Self, Box<dyn Error>> {
		Ok(Self {
			inner: AddrIncoming::bind(addr)?,
			tls: native_tls::TlsAcceptor::builder(identity)
				.min_protocol_version(Some(Protocol::Tlsv12))
				.build()?
				.into(),
		})
	}

	#[allow(unsafe_code)]
	/// Returns a pinned mutable reference to the [`inner`](Self#structfield.inner) field
	#[inline]
	fn pin_inner(self: Pin<&mut Self>) -> Pin<&mut AddrIncoming> {
		// SAFETY: `inner` is pinned when `self` is.
		unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
	}
}
impl Accept for TlsAddrIncoming {
	type Conn = TlsStream;
	type Error = TlsAcceptError;

	fn poll_accept(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
		/// Extracts the value out of `Poll::Ready(Some(Ok(_)))`
		macro_rules! try_poll {
			($poll:expr) => {
				match $poll {
					Poll::Ready(Some(Ok(value))) => value,
					Poll::Ready(Some(Err(err))) => {
						return Poll::Ready(Some(Err(err)));
					}
					Poll::Ready(None) => {
						return Poll::Ready(None);
					}
					Poll::Pending => {
						return Poll::Pending;
					}
				}
			};
		}

		let stream = try_poll!(self
			.as_mut()
			.pin_inner()
			.poll_accept(cx)
			.map_err(TlsAcceptError::from));
		let tls_stream = try_poll!(std::pin::pin!(self.tls.accept(stream))
			.poll(cx)
			.map_err(TlsAcceptError::from)
			.map(Some));
		Poll::Ready(Some(Ok(TlsStream(tls_stream))))
	}
}

/// Wrapper around [`tokio_native_tls::TlsStream<AddrStream>`] that implements [`Connected`]
#[repr(transparent)]
pub(crate) struct TlsStream(tokio_native_tls::TlsStream<AddrStream>);
impl TlsStream {
	#[allow(unsafe_code)]
	/// Returns a pinned mutable reference to the wrapped stream
	#[inline]
	fn pin_inner(self: Pin<&mut Self>) -> Pin<&mut tokio_native_tls::TlsStream<AddrStream>> {
		// SAFETY: The wrapped stream is pinned when `self` is.
		unsafe { self.map_unchecked_mut(|this| &mut this.0) }
	}
}
impl AsyncRead for TlsStream {
	#[inline]
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<io::Result<()>> {
		self.pin_inner().poll_read(cx, buf)
	}
}
impl AsyncWrite for TlsStream {
	#[inline]
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<io::Result<usize>> {
		self.pin_inner().poll_write(cx, buf)
	}

	#[inline]
	fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
		self.pin_inner().poll_flush(cx)
	}

	#[inline]
	fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
		self.pin_inner().poll_shutdown(cx)
	}

	#[inline]
	fn is_write_vectored(&self) -> bool {
		self.0.is_write_vectored()
	}

	#[inline]
	fn poll_write_vectored(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		bufs: &[IoSlice<'_>],
	) -> Poll<io::Result<usize>> {
		self.pin_inner().poll_write_vectored(cx, bufs)
	}
}
impl Connected<&TlsStream> for SocketAddr {
	#[inline]
	fn connect_info(target: &TlsStream) -> Self {
		Self::connect_info(target.0.get_ref().get_ref().get_ref())
	}
}

/// Errors that might occur in [`TlsAddrIncoming::poll_accept`]
#[derive(Debug)]
pub(crate) enum TlsAcceptError {
	/// IO error
	Io(io::Error),
	/// TLS error
	Tls(native_tls::Error),
}
impl From<io::Error> for TlsAcceptError {
	#[inline]
	fn from(err: io::Error) -> Self {
		Self::Io(err)
	}
}
impl From<native_tls::Error> for TlsAcceptError {
	#[inline]
	fn from(err: native_tls::Error) -> Self {
		Self::Tls(err)
	}
}
impl Display for TlsAcceptError {
	#[inline]
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Io(err) => Display::fmt(err, f),
			Self::Tls(err) => Display::fmt(err, f),
		}
	}
}
impl Error for TlsAcceptError {
	#[inline]
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Io(err) => Some(err),
			Self::Tls(err) => Some(err),
		}
	}
}
