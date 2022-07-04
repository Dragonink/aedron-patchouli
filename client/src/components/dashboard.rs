use crate::router::{DashboardRoute, Route};
use aedron_patchouli_common::users::{User, UserCookie};
use std::{convert::Infallible, ops::Deref, str::FromStr};
use sycamore::{component::Prop, prelude::*};
use wasm_bindgen::JsCast;

#[component]
pub(super) fn Dashboard<G: Html>(cx: Scope, route: &DashboardRoute) -> View<G> {
	use sycamore::builder::prelude::*;

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
			DashboardRoute::Users if user.is_admin => Users(cx),
			_ => t("not found"),
		},
	])
}

#[component]
fn Profile<'a, G: Html>(cx: Scope<'a>, user: &'a UserCookie) -> View<G> {
	use aedron_patchouli_common::users::API_ENDPOINT;
	use sycamore::builder::prelude::*;
	use web_sys::{Event, HtmlInputElement};

	let new_passwd = create_signal(cx, String::new());
	let confirm_new_passwd = create_signal(cx, String::new());

	main()
		.c(h1().t("Profile"))
		.c(form()
			.attr("method", "put")
			.attr("action", format!("{API_ENDPOINT}/me"))
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
			.c(button().attr("type", "submit").t("Save changes")))
		.c(form()
			.attr("method", "post")
			.attr("action", format!("{API_ENDPOINT}/me/passwd"))
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
			.c(button().attr("type", "submit").t("Change password")))
		.view(cx)
}

#[component]
fn Users<G: Html>(cx: Scope) -> View<G> {
	use aedron_patchouli_common::users::{User, API_ENDPOINT};
	use sycamore::{
		builder::prelude::*,
		suspense::{Suspense, SuspenseProps},
	};
	use web_sys::Event;

	let dialog_ref = create_node_ref(cx);
	let managed_user_sig = create_signal(cx, None);
	let users = create_signal(cx, Vec::<User>::new());
	let fetched_users_props = FetchedUsersProps {
		dialog_ref,
		managed_user_sig,
		users,
	};
	let managed_user_props = ManageUserProps {
		dialog_ref,
		managed_user_sig,
		users,
	};
	let props = SuspenseProps::builder()
		.children(Children::new(cx, move |cx| {
			FetchedUsers(cx, fetched_users_props)
		}))
		.fallback(t("Loading..."))
		.build();

	main()
		.c(h1().t("Users"))
		.c(form()
			.attr("method", "post")
			.attr("action", API_ENDPOINT)
			.on("submit", move |ev: Event| {
				let req = super::form_build_req(&ev);
				sycamore::futures::spawn_local_scoped(cx, async move {
					match crate::fetch_api::<User>(&req).await.unwrap() {
						Ok(user) => {
							users.modify().push(user);
						}
						Err(_status) => {
							web_sys::window()
								.unwrap()
								.alert_with_message(
									"Something went wrong on the server.\nPlease try again.",
								)
								.unwrap();
						}
					}
				});
			})
			.c(label()
				.t("Username")
				.c(input().attr("name", "name").bool_attr("required", true)))
			.c(label().t("Password").c(input()
				.attr("name", "passwd")
				.attr("type", "password")
				.bool_attr("required", true)))
			.c(button().attr("type", "submit").t("Create user")))
		.c(Suspense(cx, props))
		.c(ManageUser(cx, managed_user_props))
		.view(cx)
}

#[derive(Clone, Copy, Prop)]
struct FetchedUsersProps<'a, G: Html> {
	dialog_ref: &'a NodeRef<G>,
	managed_user_sig: &'a Signal<Option<User>>,
	users: &'a Signal<Vec<User>>,
}
#[component]
async fn FetchedUsers<'a, G: Html>(cx: Scope<'a>, props: FetchedUsersProps<'a, G>) -> View<G> {
	use aedron_patchouli_common::users::API_ENDPOINT;
	use sycamore::builder::prelude::*;
	use web_sys::{HtmlDialogElement, Request};

	let FetchedUsersProps {
		dialog_ref,
		managed_user_sig,
		users,
	} = props;

	let req = Request::new_with_str(API_ENDPOINT).unwrap();
	users.set(crate::fetch_api(&req).await.unwrap().unwrap());
	let props = IndexedProps::builder()
		.iterable(users)
		.view(move |cx, user| {
			let name = create_ref(cx, user.name.clone());

			li().t(name)
				.c(button()
					.on("click", move |_| {
						let dialog_el = dialog_ref
							.get::<DomNode>()
							.unchecked_into::<HtmlDialogElement>();
						managed_user_sig.modify().replace(user.clone());
						dialog_el.show_modal().unwrap();
					})
					.t("Manage"))
				.view(cx)
		})
		.build();

	ul().c(Indexed(cx, props)).view(cx)
}

#[derive(Clone, Copy, Prop)]
struct ManageUserProps<'a, G: Html> {
	dialog_ref: &'a NodeRef<G>,
	managed_user_sig: &'a ReadSignal<Option<User>>,
	users: &'a Signal<Vec<User>>,
}
#[component]
fn ManageUser<'a, G: Html>(cx: Scope<'a>, props: ManageUserProps<'a, G>) -> View<G> {
	use aedron_patchouli_common::users::API_ENDPOINT;
	use sycamore::builder::prelude::*;
	use web_sys::{
		Event, FormData, Headers, HtmlButtonElement, HtmlDialogElement, HtmlFormElement, Request,
		RequestInit, UrlSearchParams,
	};

	#[derive(Debug, Clone, PartialEq, Eq)]
	enum DialogValue {
		Cancel,
		Delete,
		Update(String),
	}
	impl DialogValue {
		pub const CANCEL: &'static str = "cancel";
		pub const DELETE: &'static str = "delete";
	}
	impl FromStr for DialogValue {
		type Err = Infallible;

		fn from_str(s: &str) -> Result<Self, Self::Err> {
			Ok(if s == Self::CANCEL {
				Self::Cancel
			} else if s == Self::DELETE {
				Self::Delete
			} else {
				Self::Update(s.to_string())
			})
		}
	}

	let ManageUserProps {
		dialog_ref,
		managed_user_sig,
		users,
	} = props;

	dialog()
		.bind_ref(dialog_ref.clone())
		.on("close", move |_| {
			let user = managed_user_sig.get();
			let user = if let Some(user) = user.deref() {
				user
			} else {
				return;
			};

			let mut req = RequestInit::new();
			match dialog_ref
				.get::<DomNode>()
				.unchecked_into::<HtmlDialogElement>()
				.return_value()
				.parse()
				.unwrap()
			{
				DialogValue::Cancel => {
					return;
				}
				DialogValue::Delete => {
					if !web_sys::window()
						.unwrap()
						.confirm_with_message(&format!(
							"Are you sure you want to delete the user {:?}?",
							user.name
						))
						.unwrap()
					{
						return;
					}
					req.method("DELETE");
				}
				DialogValue::Update(data) => {
					let headers = Headers::new().unwrap();
					headers
						.append("content-type", "application/x-www-form-urlencoded")
						.unwrap();
					req.method("PUT").headers(&headers).body(Some(&data.into()));
				}
			}
			let req = Request::new_with_str_and_init(&format!("{API_ENDPOINT}/{}", user.id), &req)
				.unwrap();
			sycamore::futures::spawn_local_scoped(cx, async move {
				match crate::send_api(&req).await.unwrap() {
					204 => users
						.modify()
						.retain(|user| user.ne(managed_user_sig.get().as_ref().as_ref().unwrap())),
					405 => web_sys::window()
						.unwrap()
						.alert_with_message("Cannot delete this user.")
						.unwrap(),
					_ => web_sys::window()
						.unwrap()
						.alert_with_message(
							"Something went wrong on the server.\nPlease try again.",
						)
						.unwrap(),
				}
			});
		})
		.c(form()
			.attr("method", "dialog")
			.c(label().t("Username").c(input()
				.attr("name", "name")
				.bool_attr("required", true)
				.dyn_attr("value", || {
					managed_user_sig
						.get()
						.as_ref()
						.as_ref()
						.map(|user| user.name.clone())
				})))
			.c(button()
				.attr("type", "submit")
				.bool_attr("formnovalidate", true)
				.attr("value", DialogValue::CANCEL)
				.t("Cancel"))
			.c(button()
				.attr("type", "submit")
				.bool_attr("formnovalidate", true)
				.attr("value", DialogValue::DELETE)
				.t("Delete user"))
			.c(button()
				.attr("type", "submit")
				.on("click", |ev: Event| {
					let btn = ev.target().unwrap().unchecked_into::<HtmlButtonElement>();
					let form = btn
						.parent_element()
						.unwrap()
						.unchecked_into::<HtmlFormElement>();
					let form_data = FormData::new_with_form(&form).unwrap();
					let search_params = UrlSearchParams::new_with_str_sequence_sequence(&form_data)
						.unwrap()
						.to_string();
					btn.set_value(&String::from(&search_params));
				})
				.t("Update user")))
		.view(cx)
}
