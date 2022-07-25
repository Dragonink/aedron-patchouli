use crate::router::{DashboardRoute, Route};
use aedron_patchouli_common::users::UserCookie;
use std::{ops::Deref, str::FromStr};
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
			.c(nav()
				.c(a().attr("href", "/").t("Close dashboard"))
				.c(ul().c(View::new_fragment(
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

#[derive(Prop)]
struct ManageDialogProps<'a, G: Html, T, F1, F2>
where
	T: Clone + PartialEq,
	F1: Fn(&T) -> &str,
	F2: Fn(&T) -> String,
{
	item_desc: &'a str,
	dialog_ref: &'a NodeRef<G>,
	items: &'a Signal<Vec<T>>,
	managed_item_sig: &'a ReadSignal<Option<T>>,
	children: Children<'a, G>,
	item_name: F1,
	req_input: F2,
}
#[component]
fn ManageDialog<'a, G: Html, T, F1, F2>(
	cx: Scope<'a>,
	props: ManageDialogProps<'a, G, T, F1, F2>,
) -> View<G>
where
	T: Clone + PartialEq,
	F1: Fn(&T) -> &str + 'a,
	F2: Fn(&T) -> String + 'a,
{
	use std::convert::Infallible;
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
		pub const CANCEL: &'static str = "__cancel__";
		pub const DELETE: &'static str = "__delete__";
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

	let ManageDialogProps {
		item_desc,
		dialog_ref,
		items,
		managed_item_sig,
		children,
		item_name,
		req_input,
	} = props;

	let delete_label = create_ref(cx, format!("Delete {item_desc}"));
	let update_label = create_ref(cx, format!("Update {item_desc}"));

	dialog()
		.bind_ref(dialog_ref.clone())
		.on("close", move |_| {
			let item = managed_item_sig.get();
			let item = if let Some(item) = item.deref() {
				item
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
							"Are you sure you want to delete the {item_desc} {:?}?",
							item_name(item)
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
			let req = Request::new_with_str_and_init(&req_input(item), &req).unwrap();
			sycamore::futures::spawn_local_scoped(cx, async move {
				match crate::send_api(&req).await.unwrap() {
					204 => items
						.modify()
						.retain(|item| item.ne(managed_item_sig.get().as_ref().as_ref().unwrap())),
					405 => web_sys::window()
						.unwrap()
						.alert_with_message(&format!("Cannot delete this {item_desc}."))
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
			.c(children.call(cx))
			.c(button()
				.attr("type", "submit")
				.bool_attr("formnovalidate", true)
				.attr("value", DialogValue::CANCEL)
				.t("Cancel"))
			.c(button()
				.attr("type", "submit")
				.bool_attr("formnovalidate", true)
				.attr("value", DialogValue::DELETE)
				.t(delete_label))
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
				.t(update_label)))
		.view(cx)
}
