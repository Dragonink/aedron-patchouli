//! Provides UI components
#![allow(
	unreachable_pub,
	clippy::empty_structs_with_brackets,
	clippy::missing_docs_in_private_items
)]

use crate::RequestClient;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde_json::Value;
use std::collections::HashMap;

/// Main component of the application
#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();

	view! {
		<Meta name="application-name" content="Aedron Patchouli" />
		<Meta name="description" content="Friendly media server" />
		<Meta name="color-scheme" content="dark" />
		<Title formatter=|text| format!("{text} — Aedron Patchouli") />

		<Router fallback=|| template! { <h1>"NOT FOUND"</h1> }.into_view()>
			<header>
				<h1>"Aedron Patchouli"</h1>
			</header>
			<main>
				<Routes>
					<Route path="/" view=LibrariesIndex />
					<Route path="/:library" view=LibraryShow />
				</Routes>
			</main>
		</Router>
	}
}

fn fetch_fallback(errors: RwSignal<Errors>) -> impl IntoView {
	view! {
		<p>
			<b>"Could not fetch data because of the following errors:"</b>
			<ul>
				<For
					each=move || errors.get()
					key=|(key, _)| key.clone()
					children=|(_, err)| template! {
						<li>{err.to_string()}</li>
					}
				/>
			</ul>
		</p>
	}
}

#[component]
fn LibrariesIndex() -> impl IntoView {
	let client = use_context::<RequestClient>();
	let libraries = create_resource::<_, Result<HashMap<String, String>, ServerFnError>, _>(
		|| (),
		move |()| {
			let client = client.clone();
			async move {
				Ok(if let Some(client) = client {
					client.get("/api/libraries").send().await?.json().await?
				} else {
					Default::default()
				})
			}
		},
	);

	view! {
		<Suspense fallback=|| template! { <p>"Loading..."</p> }>
			<ErrorBoundary fallback=fetch_fallback>
				<nav><ul>
					{move || libraries.get().map(|libraries| libraries.map(|libraries| {
						libraries.iter()
							.map(|(url, display)| template! {
								<li>
									<a href=format!("/{url}")>{display}</a>
								</li>
							})
							.collect_view()
					}))}
				</ul></nav>
			</ErrorBoundary>
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

	let client = use_context::<RequestClient>();
	let library = create_resource::<_, Result<Vec<HashMap<String, Value>>, ServerFnError>, _>(
		library,
		move |library| {
			let client = client.clone();
			async move {
				Ok(if let Some(client) = client {
					client
						.get(&format!("/api/libraries/{library}"))
						.send()
						.await?
						.json()
						.await?
				} else {
					Default::default()
				})
			}
		},
	);

	view! {
		<Suspense fallback=|| template! { <p>"Loading ..."</p> }>
			<ErrorBoundary fallback=fetch_fallback>
				{move || library.get().transpose().map(|library| view! {
					<ul>
						<For
							each=move || library.clone().unwrap_or_default()
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
				})}
			</ErrorBoundary>
		</Suspense>
	}
}
