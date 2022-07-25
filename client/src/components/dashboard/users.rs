use aedron_patchouli_common::users::User;
// use std::{ops::Deref, str::FromStr};
use sycamore::{component::Prop, prelude::*};
// use wasm_bindgen::JsCast;

#[component]
pub(super) fn Users<G: Html>(cx: Scope) -> View<G> {
	use super::{ManageDialog, ManageDialogProps};
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
	let props = SuspenseProps::builder()
		.children(Children::new(cx, move |cx| {
			FetchedUsers(cx, fetched_users_props)
		}))
		.fallback(t("Loading..."))
		.build();
	let dialog_props = ManageDialogProps::builder()
		.item_desc("user")
		.dialog_ref(dialog_ref)
		.items(users)
		.managed_item_sig(managed_user_sig)
		.children(Children::new(cx, |cx| {
			label()
				.t("Username")
				.c(input()
					.attr("name", "name")
					.bool_attr("required", true)
					.dyn_attr("value", || {
						managed_user_sig
							.get()
							.as_ref()
							.as_ref()
							.map(|user| user.name.clone())
					}))
				.view(cx)
		}))
		.item_name(|user| &user.name)
		.req_input(|user| format!("{API_ENDPOINT}/{}", user.id))
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
		.c(ManageDialog(cx, dialog_props))
		.view(cx)
}

#[derive(Prop)]
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
