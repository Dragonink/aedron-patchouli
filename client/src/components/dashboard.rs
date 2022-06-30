use crate::router::DashboardRoute;
use sycamore::prelude::*;

#[component]
pub(super) fn Dashboard<G: Html>(cx: Scope, route: &DashboardRoute) -> View<G> {
	use sycamore::builder::prelude::*;

	fragment([
		header()
			.c(nav().c(ul().c(View::new_fragment(
				enum_iterator::all::<DashboardRoute>()
					.filter(|route| route.ne(&DashboardRoute::NotFound))
					.map(|route| li().c(a().attr("href", route.to_string())).view(cx))
					.collect(),
			))))
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
