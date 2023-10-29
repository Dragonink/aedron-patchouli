//! Provides TLS capabilities

use axum::extract::connect_info::Connected;
use hyper::server::{
	accept::Accept,
	conn::{AddrIncoming, AddrStream},
};
use hyper_rustls::{acceptor::TlsStream, TlsAcceptor};
use rustls::{Certificate, PrivateKey};
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;
use std::{
	fs::File,
	io::{self, BufReader, IoSlice, Write},
	net::SocketAddr,
	path::Path,
	pin::Pin,
	task::{Context, Poll},
};
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// TLS cryptographic identity
pub(crate) struct Identity {
	/// Private key data
	pub(crate) key: PrivateKey,
	/// Certificate chain data
	pub(crate) cert_chain: Vec<Certificate>,
}
impl Identity {
	/// Constructs a new instance from a private key and a certificate files
	pub(crate) fn read(key: &Path, certificate: &Path) -> io::Result<Self> {
		let mut key_file = BufReader::new(File::open(key)?);
		let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_file)?.into_iter();
		let key = keys.next().ok_or_else(|| {
			io::Error::new(io::ErrorKind::InvalidData, "Key file contains no key data")
		})?;
		if keys.next().is_some() {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"Key file contains more than one key",
			));
		}

		let mut cert_file = BufReader::new(File::open(certificate)?);
		let cert_chain = rustls_pemfile::certs(&mut cert_file)?;
		if cert_chain.is_empty() {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"Certificate file contains no certificate data",
			));
		}

		Ok(Self {
			key: PrivateKey(key),
			cert_chain: cert_chain.into_iter().map(Certificate).collect(),
		})
	}

	/// Generates a new cryptographic identity using [`rcgen`],
	/// then writes the private key and certificate in the given files
	///
	/// # Panics
	/// This function panics if an [`RcgenError`](rcgen::RcgenError) occurs.
	pub(crate) fn generate_write(
		subject_alt_names: Vec<String>,
		key: &Path,
		certificate: &Path,
	) -> io::Result<Self> {
		let mut params = rcgen::CertificateParams::new(subject_alt_names);
		params.not_before = OffsetDateTime::now_utc();
		let cert = rcgen::Certificate::from_params(params).unwrap();

		std::fs::write(certificate, cert.serialize_pem().unwrap())?;
		let mut key_file = File::create(key)?;
		let mut perms = key_file.metadata()?.permissions();
		#[cfg(unix)]
		perms.set_mode(0o600);
		key_file.set_permissions(perms)?;
		key_file.write_all(cert.serialize_private_key_pem().as_bytes())?;

		// NOTE: The `Certificate::serialize_*` functions actually generate the certificate.
		// Thus, calling multiple times the serializing functions will result in different certificates.
		// See https://github.com/rustls/rcgen/issues/62
		Self::read(key, certificate)
	}
}
impl Zeroize for Identity {
	fn zeroize(&mut self) {
		self.key.0.zeroize();
		self.cert_chain.iter_mut().for_each(|cert| cert.0.zeroize());
		self.cert_chain.clear();
	}
}
impl Drop for Identity {
	fn drop(&mut self) {
		self.zeroize();
	}
}
impl ZeroizeOnDrop for Identity {}

/// Wrapper around [`TlsAcceptor`] such that [`Accept::Conn`] implements [`Connected`]
#[repr(transparent)]
pub(crate) struct ConnectedTlsAcceptor(pub(crate) TlsAcceptor);
impl ConnectedTlsAcceptor {
	/// Constructs a new instance from a stream of connections and a TLS identity
	pub(crate) fn new(incoming: AddrIncoming, identity: &Identity) -> Result<Self, rustls::Error> {
		Ok(Self(
			TlsAcceptor::builder()
				.with_single_cert(identity.cert_chain.clone(), identity.key.clone())?
				.with_all_versions_alpn()
				.with_incoming(incoming),
		))
	}

	#[allow(unsafe_code)]
	/// Returns a pinned mutable reference to the wrapped stream
	#[inline]
	fn pin_inner(self: Pin<&mut Self>) -> Pin<&mut TlsAcceptor> {
		// SAFETY: The wrapped acceptor is pinned when `self` is.
		unsafe { self.map_unchecked_mut(|this| &mut this.0) }
	}
}
impl From<TlsAcceptor> for ConnectedTlsAcceptor {
	#[inline]
	fn from(acceptor: TlsAcceptor) -> Self {
		Self(acceptor)
	}
}
impl Accept for ConnectedTlsAcceptor {
	type Conn = ConnectedTlsStream;
	type Error = <TlsAcceptor as Accept>::Error;

	#[inline]
	fn poll_accept(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
		self.pin_inner()
			.poll_accept(cx)
			.map_ok(ConnectedTlsStream::from)
	}
}

/// Wrapper around [`TlsStream<AddrStream>`] that implements [`Connected`]
#[repr(transparent)]
pub(crate) struct ConnectedTlsStream(pub(crate) TlsStream<AddrStream>);
impl ConnectedTlsStream {
	#[allow(unsafe_code)]
	/// Returns a pinned mutable reference to the wrapped stream
	#[inline]
	fn pin_inner(self: Pin<&mut Self>) -> Pin<&mut TlsStream<AddrStream>> {
		// SAFETY: The wrapped stream is pinned when `self` is.
		unsafe { self.map_unchecked_mut(|this| &mut this.0) }
	}
}
impl From<TlsStream<AddrStream>> for ConnectedTlsStream {
	#[inline]
	fn from(stream: TlsStream<AddrStream>) -> Self {
		Self(stream)
	}
}
impl AsyncRead for ConnectedTlsStream {
	#[inline]
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<io::Result<()>> {
		self.pin_inner().poll_read(cx, buf)
	}
}
impl AsyncWrite for ConnectedTlsStream {
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
impl Connected<&ConnectedTlsStream> for SocketAddr {
	#[inline]
	fn connect_info(target: &ConnectedTlsStream) -> Self {
		Self::connect_info(target.0.io().unwrap())
	}
}
