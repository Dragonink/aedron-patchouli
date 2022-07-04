use aedron_patchouli_common::libraries::{LibraryConfig, PartialLibrary};
use std::{convert::Infallible, ops::Deref, str::FromStr};
use sycamore::{component::Prop, prelude::*};
use wasm_bindgen::JsCast;

#[component]
pub(super) fn Libraries<G: Html>(cx: Scope) -> View<G> {
	use sycamore::{
		builder::prelude::*,
		component::Children,
		suspense::{Suspense, SuspenseProps},
	};

	let dialog_ref = create_node_ref(cx);
	let managed_library_sig = create_signal(cx, None);
	let libraries = create_signal(cx, Vec::<PartialLibrary>::new());
	let fetched_libraries_props = FetchedLibrariesProps {
		dialog_ref,
		managed_library_sig,
		libraries,
	};
	let managed_library_props = ManageLibraryProps {
		dialog_ref,
		managed_library_sig,
	};
	let props = SuspenseProps::builder()
		.children(Children::new(cx, move |cx| {
			FetchedLibraries(cx, fetched_libraries_props)
		}))
		.fallback(t("Loading..."))
		.build();

	main()
		.c(Suspense(cx, props))
		.c(CreateLibrary(cx, libraries))
		.c(ManageLibrary(cx, managed_library_props))
		.view(cx)
}

#[derive(Clone, Copy, Prop)]
struct FetchedLibrariesProps<'a, G: Html> {
	dialog_ref: &'a NodeRef<G>,
	managed_library_sig: &'a Signal<Option<LibraryConfig>>,
	libraries: &'a Signal<Vec<PartialLibrary>>,
}
#[component]
async fn FetchedLibraries<'a, G: Html>(
	cx: Scope<'a>,
	props: FetchedLibrariesProps<'a, G>,
) -> View<G> {
	use aedron_patchouli_common::libraries::API_ENDPOINT;
	use sycamore::builder::prelude::*;
	use web_sys::{HtmlDialogElement, Request};

	let FetchedLibrariesProps {
		dialog_ref,
		managed_library_sig,
		libraries,
	} = props;

	let req = Request::new_with_str(API_ENDPOINT).unwrap();
	libraries.set(crate::fetch_api(&req).await.unwrap().unwrap());
	let props = IndexedProps::builder()
		.iterable(libraries)
		.view(move |cx, library| {
			let name = create_ref(cx, library.name.clone());

			li().c(a().attr("href", format!("/library/{}", library.id)).t(name))
				.c(button()
					.on("click", move |_| {
						let req = Request::new_with_str(&format!(
							"{API_ENDPOINT}/{}?config=true",
							library.id
						))
						.unwrap();
						let dialog_el = dialog_ref
							.get::<DomNode>()
							.unchecked_into::<HtmlDialogElement>();
						sycamore::futures::spawn_local_scoped(cx, async move {
							let config = crate::fetch_api(&req).await.unwrap().unwrap();
							managed_library_sig.modify().replace(config);
							dialog_el.show_modal().unwrap();
						});
					})
					.t("Manage"))
				.view(cx)
		})
		.build();

	ul().c(Indexed(cx, props)).view(cx)
}

#[component]
fn LibraryFormFields<G: Html>(cx: Scope, library: Option<LibraryConfig>) -> View<G> {
	use aedron_patchouli_common::libraries::LibraryKind;
	use sycamore::builder::prelude::*;
	use web_sys::{Event, HtmlInputElement};

	let library = create_ref(cx, library);
	let paths_sig = create_signal(
		cx,
		library
			.as_ref()
			.map(|library| (0..=library.paths.len()).collect())
			.unwrap_or_else(|| vec![0]),
	);
	let indexed_props = IndexedProps::builder()
		.iterable(paths_sig)
		.view(move |cx, index| {
			let value = create_signal(
				cx,
				library
					.as_ref()
					.map(|library| library.paths.get(index).cloned().unwrap_or_default())
					.unwrap_or_default(),
			);

			input()
				.bind_value(value)
				.dyn_attr("name", || (!value.get().is_empty()).then(|| "paths"))
				.bool_attr("required", index.eq(paths_sig.get().first().unwrap()))
				.on("input", move |_| {
					// On input, if element is the last, append a new input
					let paths = paths_sig.get();
					let last_index = paths.last().unwrap();
					if index.eq(last_index) {
						paths_sig.modify().push(*last_index + 1);
					}
				})
				.on("blur", move |ev: Event| {
					// On blur, if element is not the last and value not empty, remove element
					let paths = paths_sig.get();
					let last_index = paths.last().unwrap();
					if index < *last_index {
						let val = ev
							.target()
							.unwrap()
							.unchecked_into::<HtmlInputElement>()
							.value();
						if val.is_empty() {
							paths_sig.modify().retain(|&i| i != index);
						}
					}
				})
				.view(cx)
		})
		.build();

	fragment([
		label()
			.t("Name")
			.c(input()
				.attr("name", "name")
				.bool_attr("required", true)
				.attr(
					"value",
					library
						.as_ref()
						.map(|library| library.name.clone())
						.unwrap_or_default(),
				))
			.view(cx),
		label()
			.t("Type")
			.c(select()
				.attr("name", "kind")
				.bool_attr("required", true)
				.bool_attr("disabled", library.is_some())
				.c(option().attr("value", "").t("— Please select a type —"))
				.c(View::new_fragment(
					enum_iterator::all::<LibraryKind>()
						.map(|var| {
							let var_s = create_ref(cx, format!("{var:?}"));

							option()
								.attr("value", var_s)
								.bool_attr(
									"selected",
									library
										.as_ref()
										.map(|library| library.kind == var)
										.unwrap_or_default(),
								)
								.t(var_s)
								.view(cx)
						})
						.collect(),
				)))
			.view(cx),
		fieldset()
			.c(legend().t("Paths"))
			.c(Indexed(cx, indexed_props))
			.view(cx),
	])
}

#[component]
fn CreateLibrary<'a, G: Html>(
	cx: Scope<'a>,
	libraries: &'a Signal<Vec<PartialLibrary>>,
) -> View<G> {
	use aedron_patchouli_common::libraries::{LibraryConfig, API_ENDPOINT};
	use sycamore::builder::prelude::*;
	use web_sys::Event;

	form()
		.attr("method", "post")
		.attr("action", API_ENDPOINT)
		.on("submit", move |ev: Event| {
			let req = super::form_build_req(&ev);
			sycamore::futures::spawn_local_scoped(cx, async move {
				match crate::fetch_api::<LibraryConfig>(&req).await.unwrap() {
					Ok(lib) => {
						libraries.modify().push(lib.into());
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
		.c(LibraryFormFields(cx, None))
		.c(button().attr("type", "submit").t("Create library"))
		.view(cx)
}

#[derive(Prop)]
struct ManageLibraryProps<'a, G: Html> {
	dialog_ref: &'a NodeRef<G>,
	managed_library_sig: &'a ReadSignal<Option<LibraryConfig>>,
}
#[component]
fn ManageLibrary<'a, G: Html>(cx: Scope<'a>, props: ManageLibraryProps<'a, G>) -> View<G> {
	use aedron_patchouli_common::libraries::API_ENDPOINT;
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

	let ManageLibraryProps {
		dialog_ref,
		managed_library_sig,
	} = props;

	dialog()
		.bind_ref(dialog_ref.clone())
		.on("close", move |_| {
			let library = managed_library_sig.get();
			let library = if let Some(library) = library.deref() {
				library
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
							"Are you sure you want to delete the library {:?}?\n(Worry not, your media files will not be deleted)",
							library.name
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
			let req =
				Request::new_with_str_and_init(&format!("{API_ENDPOINT}/{}", library.id), &req)
					.unwrap();
			let location = web_sys::window().unwrap().location();
			sycamore::futures::spawn_local_scoped(cx, async move {
				let status = crate::send_api(&req).await.unwrap();
				if status == 204 {
					let _ = location.reload();
				}
			});
		})
		.c(form()
			.attr("method", "dialog")
			.dyn_c(move || LibraryFormFields(cx, managed_library_sig.get().as_ref().clone()))
			.c(button()
				.attr("type", "submit")
				.bool_attr("formnovalidate", true)
				.attr("value", DialogValue::CANCEL)
				.t("Cancel"))
			.c(button()
				.attr("type", "submit")
				.bool_attr("formnovalidate", true)
				.attr("value", DialogValue::DELETE)
				.t("Delete library"))
			.c(button()
				.attr("type", "submit")
				.on("click", |ev: Event| {
					let btn = ev
						.target()
						.unwrap()
						.unchecked_into::<HtmlButtonElement>();
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
				.t("Update library")))
		.view(cx)
}
