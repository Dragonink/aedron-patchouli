use sycamore::prelude::*;
use wasm_bindgen::JsCast;

mod dashboard;
mod libraries;
mod library;
mod media;

#[component]
pub(crate) fn App<G: Html>(cx: Scope) -> View<G> {
	use crate::router::Route;
	use aedron_patchouli_common::users::UserCookie;
	use sycamore::builder::prelude::*;
	use sycamore_router::{HistoryIntegration, Router, RouterProps};
	use wasm_bindgen::{closure::Closure, JsValue};
	use web_sys::{MutationObserver, MutationObserverInit, MutationRecord};

	Router(
		cx,
		RouterProps::new(
			HistoryIntegration::new(),
			|cx, route: &ReadSignal<Route>| {
				let observer = MutationObserver::new(
					Closure::once_into_js(|mutations: Box<[JsValue]>, this: JsValue| {
						let this: MutationObserver = this.unchecked_into();
						'mutations: for rec in mutations.iter() {
							let rec: &MutationRecord = rec.unchecked_ref();
							let added = rec.added_nodes();
							for i in 0..added.length() {
								if let Some(node) = added.item(i) {
									if node.node_name() == "BODY" {
										web_sys::window()
											.and_then(|window| window.document())
											.unwrap()
											.set_body(Some(node.unchecked_ref()));
										break 'mutations;
									}
								}
							}
						}
						this.disconnect();
					})
					.unchecked_ref(),
				)
				.unwrap();
				let mut options = MutationObserverInit::new();
				options.child_list(true);
				observer
					.observe_with_options(
						&web_sys::window()
							.and_then(|window| window.document())
							.and_then(|document| document.document_element())
							.unwrap(),
						&options,
					)
					.unwrap();

				body()
					.dyn_attr("id", || {
						let route = route.get();
						Some(format!("{:?}", route.as_ref()))
					})
					.dyn_c(move || {
						let route = route.get();
						match route.as_ref() {
							Route::Dashboard(route) => dashboard::Dashboard(cx, route),
							route => fragment([
								header()
									.c(a().attr("href", "/dashboard/").t("Connected as ").c(b()
										.dyn_t(move || {
											let user: &UserCookie = use_context(cx);
											create_ref(cx, user.name.clone())
										})))
									.view(cx),
								match route {
									Route::Home => libraries::Libraries(cx),
									&Route::Library { id } => library::Library(cx, id),
									&Route::Media { library, media } => {
										media::Media(cx, media::MediaProps { library, media })
									}
									Route::NotFound => t("not found"),
									Route::Dashboard(_) => unreachable!(),
								},
							]),
						}
					})
					.view(cx)
			},
		),
	)
}
