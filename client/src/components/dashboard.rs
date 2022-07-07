use crate::router::{DashboardRoute, Route};
use aedron_patchouli_common::users::UserCookie;
use std::ops::Deref;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, Request};

mod libraries;
mod profile;
mod users;

fn form_build_req(ev: &Event) -> Request {
	use web_sys::{FormData, HtmlFormElement, RequestInit};

	ev.prevent_default();
	let form: HtmlFormElement = ev.target().unwrap().unchecked_into();
	let form_data = FormData::new_with_form(&form).unwrap();
	let mut req = RequestInit::new();
	req.method(&form.get_attribute("method").unwrap())
		.body(Some(form_data.deref()));
	Request::new_with_str_and_init(&form.action(), &req).unwrap()
}

#[component]
pub(super) fn Dashboard<G: Html>(cx: Scope, route: &DashboardRoute) -> View<G> {
	use libraries::Libraries;
	use profile::Profile;
	use sycamore::builder::prelude::*;
	use users::Users;

	let user: &UserCookie = use_context(cx);

	fragment([
		header()
			.c(nav().c(ul().c(View::new_fragment(
				enum_iterator::all::<DashboardRoute>()
					.filter(|route| {
						route.ne(&DashboardRoute::NotFound)
							&& (route.eq(&DashboardRoute::Profile) || user.is_admin)
					})
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
			DashboardRoute::Profile => Profile(cx, user),
			DashboardRoute::Libraries if user.is_admin => Libraries(cx),
			DashboardRoute::Users if user.is_admin => Users(cx),
			_ => t("not found"),
		},
	])
}
