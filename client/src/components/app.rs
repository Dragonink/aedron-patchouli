use sycamore::prelude::*;

#[component]
pub(crate) fn App<G: Html>(cx: Scope) -> View<G> {
	use crate::router::Route;
	use aedron_patchouli_common::users::UserCookie;
	use sycamore::builder::prelude::*;
	use sycamore_router::{HistoryIntegration, Router, RouterProps};

	Router(
		cx,
		RouterProps::new(
			HistoryIntegration::new(),
			|cx, route: &ReadSignal<Route>| {
				// let node_ref = create_node_ref(cx);
				div()
					// .bind_ref(node_ref)
					.dyn_attr("id", || {
						let route = route.get();
						Some(format!("{:?}", route.as_ref()))
					})
					.c(form()
						.attr("method", "POST")
						.attr("action", "/logout")
						.c(p().t("Connected as ").c(b().dyn_t(move || {
							let user: &UserCookie = use_context(cx);
							create_ref(cx, user.name.clone())
						})))
						.c(button().attr("type", "submit").t("Logout")))
					.dyn_c(move || {
						let route = route.get();
						match route.as_ref() {
							Route::Home => super::Libraries(cx),
							&Route::Library(id) => super::Library(cx, id),
							&Route::Media { library, media } => {
								super::Media(cx, super::MediaProps { library, media })
							}
							Route::NotFound => t("not found"),
						}
					})
					.view(cx)
			},
		),
	)
}
