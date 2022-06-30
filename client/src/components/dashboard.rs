use crate::router::{DashboardRoute, Route};
use sycamore::prelude::*;

#[component]
pub(super) fn Dashboard<G: Html>(cx: Scope, route: &DashboardRoute) -> View<G> {
	use sycamore::builder::prelude::*;

	fragment([
		header()
			.c(nav().c(ul().c(View::new_fragment(
				enum_iterator::all::<DashboardRoute>()
					.filter(|route| route.ne(&DashboardRoute::NotFound))
					.map(|route| {
						let txt = create_ref(cx, format!("{route:?}"));

						li().c(a().attr("href", Route::Dashboard(route).to_string()).t(txt))
							.view(cx)
					})
					.collect(),
			))))
			.c(form()
				.attr("method", "post")
				.attr("action", "/logout")
				.c(button().attr("type", "submit").t("Logout"))
				.view(cx))
			.view(cx),
		match route {
			DashboardRoute::Profile => Profile(cx),
			DashboardRoute::NotFound => t("not found"),
		},
	])
}

#[component]
fn Profile<G: Html>(cx: Scope) -> View<G> {
	use sycamore::builder::prelude::*;

	main().view(cx)
}
