//! Provides UI components
#![allow(
	clippy::empty_structs_with_brackets,
	clippy::missing_docs_in_private_items
)]

use gloo_net::http::RequestBuilder;
use leptos::*;
use leptos_router::*;
use serde_json::Value;
use std::collections::HashMap;

/// Main component of the application
#[component]
pub(super) fn App() -> impl IntoView {
	view! {
		<Router>
			<header>
				<h1>"Aedron Patchouli"</h1>
			</header>
			<main>
				<Routes>
					<Route path="/" view=LibrariesIndex />
					<Route path="/:library" view=LibraryShow />
					<Route path="/*any" view=|| template! { <h1>"NOT FOUND"</h1> } />
				</Routes>
			</main>
		</Router>
	}
}

#[component]
fn LibrariesIndex() -> impl IntoView {
	#[inline]
	async fn fetch_libraries() -> Result<HashMap<String, String>, gloo_net::Error> {
		RequestBuilder::new("/api/libraries")
			.header("accept", "application/json")
			.send()
			.await?
			.json()
			.await
	}
	let libraries = create_local_resource(|| (), |()| async { fetch_libraries().await.unwrap() });

	view! {
		<Suspense fallback=|| template! { <p>"Loading..."</p> }>
			<nav><ul>
				{move || with!(|libraries| libraries.as_ref().map(|libraries| {
					libraries.iter()
						.map(|(url, display)| template! {
							<li>
								<a href=format!("/{url}")>{display}</a>
							</li>
						})
						.collect_view()
				}))}
			</ul></nav>
		</Suspense>
	}
}

#[derive(Debug, PartialEq, Eq, Params)]
struct LibraryShowParams {
	library: Option<String>,
}

#[component]
fn LibraryShow() -> impl IntoView {
	let params = use_params::<LibraryShowParams>();
	let library = move || {
		with!(|params| params
			.as_ref()
			.unwrap()
			.library
			.as_ref()
			.cloned()
			.unwrap_or_else(|| unreachable!()))
	};

	async fn fetch_library(library: &str) -> Result<Vec<HashMap<String, Value>>, gloo_net::Error> {
		RequestBuilder::new(&format!("/api/libraries/{library}"))
			.header("accept", "application/json")
			.send()
			.await?
			.json()
			.await
	}
	let library = create_local_resource(library, |library| async move {
		fetch_library(&library).await.unwrap()
	});

	view! {
		<Suspense fallback=|| template! { <p>"Loading ..."</p> }>
			<ul>
				<For
					each=move || library.get().unwrap_or_default()
					key=|data| match data.get(&"path".to_owned()) {
						Some(Value::String(s)) => s.to_owned(),
						_ => unreachable!(),
					}
					children=|data| template! {
						<li>
							{format!("{data:?}")}
						</li>
					}
				/>
			</ul>
		</Suspense>
	}
}
