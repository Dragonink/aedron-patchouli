//! Provides the server's HTTP features

use axum::{
	extract::ConnectInfo,
	http::{Request, Response},
	middleware::{self, Next},
	response, Router,
};
use std::{
	fmt::{self, Display, Formatter},
	net::SocketAddr,
	time::Duration,
};
use tower::ServiceBuilder;
use tower_http::{
	classify::{ServerErrorsAsFailures, SharedClassifier},
	compression::CompressionLayer,
	normalize_path::NormalizePathLayer,
	trace::{DefaultMakeSpan, OnFailure, OnRequest, OnResponse, TraceLayer},
};
use tracing::Span;

mod assets;

/// Constructs a new configured [`Router`]
#[inline]
pub(super) fn new_router<S>() -> Router<S>
where
	S: Clone + Send + Sync + 'static,
{
	Router::<S>::new()
		.nest("/assets", assets::new_nested_router())
		.merge(assets::new_merged_router())
		.layer(
			// NOTE: Requests pass through layers top down (↓)
			ServiceBuilder::new()
				.layer(NormalizePathLayer::trim_trailing_slash())
				.layer(CustomTrace::new_layer())
				.layer(CompressionLayer::new())
				.layer(middleware::from_fn(req_to_res_extensions)),
			// NOTE: Responses pass through layers bottom up (↑)
		)
}

/// [Middleware](axum::middleware) that copies some [`Request`] extensions to the [`Response`](response::Response)
///
/// # Copied extensions
/// - [`ConnectInfo<SocketAddr>`]
async fn req_to_res_extensions<B>(request: Request<B>, next: Next<B>) -> response::Response {
	let client = request
		.extensions()
		.get::<ConnectInfo<SocketAddr>>()
		.copied();

	let mut response = next.run(request).await;
	if let Some(client) = client {
		response.extensions_mut().insert(client);
	}
	response
}

/// Gets the [`ConnectInfo<SocketAddr>`] extension from the given object
macro_rules! get_client {
	($obj:expr) => {
		$obj.extensions()
			.get::<ConnectInfo<SocketAddr>>()
			.map(|ConnectInfo(addr)| addr.to_string())
			.unwrap_or_else(|| "anonymous".to_owned())
	};
}

/// Custom implementation of [`tower_http::trace`] traits to use with [`TraceLayer`](tower_http::trace::TraceLayer)
#[derive(Debug, Default, Clone, Copy)]
struct CustomTrace;
impl CustomTrace {
	/// Constructs a new [`TraceLayer`] configured to use [`CustomTrace`]
	#[inline]
	pub(crate) fn new_layer() -> TraceLayer<
		SharedClassifier<ServerErrorsAsFailures>,
		DefaultMakeSpan,
		Self,
		Self,
		(),
		(),
		Self,
	> {
		TraceLayer::new_for_http()
			.on_request(Self)
			.on_response(Self)
			.on_body_chunk(())
			.on_eos(())
			.on_failure(Self)
	}
}
impl<B> OnRequest<B> for CustomTrace {
	fn on_request(&mut self, request: &Request<B>, span: &Span) {
		let client = get_client!(request);

		tracing::info!(parent: span, "{client} ---> {:8?} {} {}", request.version(), request.method(), request.uri());
	}
}
impl<B> OnResponse<B> for CustomTrace {
	fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
		let client = get_client!(response);

		tracing::debug!(parent: span, "{client} <--- {} (in {})", response.status(), FmtDuration(latency));
	}
}
impl<T> OnFailure<T> for CustomTrace
where
	T: Display,
{
	fn on_failure(&mut self, failure: T, latency: Duration, span: &Span) {
		tracing::warn!(parent: span, "{failure} (after {})", FmtDuration(latency));
	}
}

/// Wrapper around [`Duration`] to implement [`Display`]
#[derive(Debug, Default, Clone, Copy)]
struct FmtDuration(Duration);
impl From<Duration> for FmtDuration {
	#[inline]
	fn from(duration: Duration) -> Self {
		Self(duration)
	}
}
impl Display for FmtDuration {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let duration = self.0.as_millis();
		if duration >= 1000 {
			write!(f, "{:.3}s", self.0.as_secs_f32())
		} else {
			write!(f, "{duration}ms")
		}
	}
}
