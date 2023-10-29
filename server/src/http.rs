//! Provides the server's HTTP features

mod api;
mod assets;

use crate::AppState;
use axum::{
	extract::ConnectInfo,
	http,
	middleware::{self, Next},
	response::Response,
	Router,
};
use client::leptos;
use hyper::body::HttpBody;
use leptos_axum::LeptosRoutes;
use std::{
	fmt::{self, Display, Formatter},
	net::SocketAddr,
	time::Duration,
};
use tower::ServiceBuilder;
use tower_http::{
	classify::{ServerErrorsAsFailures, SharedClassifier},
	compression::{CompressionLayer, DefaultPredicate, Predicate},
	normalize_path::NormalizePathLayer,
	trace::{DefaultMakeSpan, OnFailure, OnRequest, OnResponse, TraceLayer},
};
use tracing::Span;

/// Constructs a new configured [`Router`]
pub(super) fn new_router(state: &AppState) -> Router<AppState> {
	let request_client = state.request_client.clone();

	Router::new()
		.nest("/api", api::new_router())
		.nest(
			&format!("/{}", state.leptos_options.site_pkg_dir),
			assets::new_router(),
		)
		.leptos_routes_with_context(
			state,
			leptos_axum::generate_route_list(client::App),
			move || {
				leptos::provide_context(request_client.clone());
			},
			client::App,
		)
		.layer(
			// NOTE: Requests pass through layers top down (↓)
			ServiceBuilder::new()
				.layer(NormalizePathLayer::trim_trailing_slash())
				.layer(CustomTrace::new_layer())
				.layer(
					CompressionLayer::new()
						.compress_when(DefaultPredicate::new().and(ProfilePredicate)),
				)
				.layer(middleware::from_fn(req_to_res_extensions)),
			// NOTE: Responses pass through layers bottom up (↑)
		)
}

/// [Middleware](axum::middleware) that copies some [`Request`] extensions to the [`Response`](response::Response)
///
/// # Copied extensions
/// - [`ConnectInfo<SocketAddr>`]
async fn req_to_res_extensions<B>(request: http::Request<B>, next: Next<B>) -> Response {
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

/// Custom implementation of [`tower_http::trace`] traits to use with [`TraceLayer`](TraceLayer)
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
	fn on_request(&mut self, request: &http::Request<B>, span: &Span) {
		let client = get_client!(request);

		tracing::trace!(parent: span, "{client} ---> {:8?} {} {}", request.version(), request.method(), request.uri());
	}
}
impl<B> OnResponse<B> for CustomTrace {
	fn on_response(self, response: &http::Response<B>, latency: Duration, span: &Span) {
		let client = get_client!(response);

		tracing::trace!(parent: span, "{client} <--- {} (in {})", response.status(), FmtDuration(latency));
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
		let duration = self.0.as_micros();
		if duration >= 1_000_000 {
			write!(f, "{:.3}s", self.0.as_secs_f32())
		} else if duration >= 1_000 {
			write!(f, "{}ms", self.0.as_millis())
		} else {
			write!(f, "{duration}μs")
		}
	}
}

/// [Compression predicate](Predicate) according to the compilation profile
#[derive(Debug, Default, Clone, Copy)]
struct ProfilePredicate;
impl Predicate for ProfilePredicate {
	#[inline]
	fn should_compress<B: HttpBody>(&self, _response: &http::Response<B>) -> bool {
		!cfg!(debug_assertions)
	}
}
