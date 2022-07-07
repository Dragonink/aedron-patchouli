use aedron_patchouli_common::users::UserCookie;
use const_format::formatcp;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub(super) fn Profile<'a, G: Html>(cx: Scope<'a>, user: &'a UserCookie) -> View<G> {
	use sycamore::builder::prelude::*;

	main()
		.c(h1().t("Profile"))
		.c(EditForm(cx, user))
		.c(ChangePasswd(cx))
		.view(cx)
}

#[component]
fn EditForm<'a, G: Html>(cx: Scope<'a>, user: &'a UserCookie) -> View<G> {
	use aedron_patchouli_common::users::API_ENDPOINT;
	use sycamore::builder::prelude::*;
	use web_sys::Event;

	form()
		.attr("method", "put")
		.attr("action", formatcp!("{API_ENDPOINT}/me"))
		.on("submit", move |ev: Event| {
			let req = super::form_build_req(&ev);
			sycamore::futures::spawn_local_scoped(cx, async move {
				let window = web_sys::window().unwrap();
				match crate::send_api(&req).await.unwrap() {
					204 => {
						let _ = window.location().reload();
					}
					_ => window
						.alert_with_message(
							"Something went wrong on the server.\nPlease try again.",
						)
						.unwrap(),
				}
			});
		})
		.c(h2().t("Edit profile"))
		.c(label().t("Username").c(input()
			.attr("name", "name")
			.attr("autocomplete", "username")
			.bool_attr("required", true)
			.attr("value", &user.name)))
		.c(button().attr("type", "submit").t("Save changes"))
		.view(cx)
}

#[component]
fn ChangePasswd<G: Html>(cx: Scope) -> View<G> {
	use aedron_patchouli_common::users::API_ENDPOINT;
	use sycamore::builder::prelude::*;
	use web_sys::{Event, HtmlInputElement};

	let new_passwd = create_signal(cx, String::new());
	let confirm_new_passwd = create_signal(cx, String::new());

	form()
		.attr("method", "post")
		.attr("action", formatcp!("{API_ENDPOINT}/me/passwd"))
		.on("submit", move |ev: Event| {
			let req = super::form_build_req(&ev);
			sycamore::futures::spawn_local_scoped(cx, async move {
				let window = web_sys::window().unwrap();
				match crate::send_api(&req).await.unwrap() {
					204 => window
						.alert_with_message("Password changed successfully.")
						.unwrap(),
					403 => window.alert_with_message("Wrong password.").unwrap(),
					_ => window
						.alert_with_message(
							"Something went wrong on the server.\nPlease try again.",
						)
						.unwrap(),
				}
			});
		})
		.c(h2().t("Change password"))
		.c(label().t("Current password").c(input()
			.attr("name", "old")
			.attr("type", "password")
			.attr("autocomplete", "current-password")
			.bool_attr("required", true)))
		.c(label().t("New password").c(input()
			.bind_value(new_passwd)
			.attr("name", "new")
			.attr("type", "password")
			.attr("autocomplete", "new-password")
			.bool_attr("required", true)))
		.c(label().t("Confirm new password").c(input()
			.bind_value(confirm_new_passwd)
			.attr("type", "password")
			.attr("autocomplete", "new-password")
			.bool_attr("required", true)
			.on("input", |ev: Event| {
				let input: HtmlInputElement = ev.target().unwrap().unchecked_into();
				if confirm_new_passwd.get() != new_passwd.get() {
					input.set_custom_validity(
						r#""Confirm new password" is different from "New password""#,
					);
					let is_valid = input.report_validity();
					debug_assert!(!is_valid);
				} else {
					input.set_custom_validity("");
				}
			})))
		.c(button().attr("type", "submit").t("Change password"))
		.view(cx)
}
