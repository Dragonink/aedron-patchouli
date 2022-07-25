use aedron_patchouli_common::libraries::LibraryConfig;
use const_format::formatcp;
use sycamore::{component::Prop, prelude::*};
use wasm_bindgen::JsCast;

#[component]
pub(super) fn Libraries<G: Html>(cx: Scope) -> View<G> {
	use super::{ManageDialog, ManageDialogProps};
	use aedron_patchouli_common::libraries::{LibraryConfig, API_ENDPOINT};
	use sycamore::{
		builder::prelude::*,
		suspense::{Suspense, SuspenseProps},
	};
	use web_sys::Event;

	let dialog_ref = create_node_ref(cx);
	let managed_library_sig = create_signal(cx, None);
	let libraries = create_signal(cx, Vec::<LibraryConfig>::new());
	let fetched_libraries_props = FetchedLibrariesProps {
		dialog_ref,
		managed_library_sig,
		libraries,
	};
	let props = SuspenseProps::builder()
		.children(Children::new(cx, move |cx| {
			FetchedLibraries(cx, fetched_libraries_props)
		}))
		.fallback(t("Loading..."))
		.build();
	let dialog_props = ManageDialogProps::builder()
		.item_desc("library")
		.dialog_ref(dialog_ref)
		.items(libraries)
		.managed_item_sig(managed_library_sig)
		.children(Children::new(cx, |cx| {
			LibraryFormFields(cx, managed_library_sig)
		}))
		.item_name(|library| &library.name)
		.req_input(|library| format!("{API_ENDPOINT}/{}", library.id))
		.build();

	main()
		.c(h1().t("Libraries"))
		.c(form()
			.attr("method", "post")
			.attr("action", API_ENDPOINT)
			.on("submit", move |ev: Event| {
				let req = super::form_build_req(&ev);
				sycamore::futures::spawn_local_scoped(cx, async move {
					match crate::fetch_api::<LibraryConfig>(&req).await.unwrap() {
						Ok(library) => {
							libraries.modify().push(library);
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
			.c(LibraryFormFields(cx, create_signal(cx, None)))
			.c(button().attr("type", "submit").t("Create library")))
		.c(Suspense(cx, props))
		.c(ManageDialog(cx, dialog_props))
		.view(cx)
}

#[derive(Prop)]
struct FetchedLibrariesProps<'a, G: Html> {
	dialog_ref: &'a NodeRef<G>,
	managed_library_sig: &'a Signal<Option<LibraryConfig>>,
	libraries: &'a Signal<Vec<LibraryConfig>>,
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

	let req = Request::new_with_str(formatcp!("{API_ENDPOINT}/?config=true")).unwrap();
	libraries.set(crate::fetch_api(&req).await.unwrap().unwrap());
	let props = IndexedProps::builder()
		.iterable(libraries)
		.view(move |cx, library| {
			let name = create_ref(cx, library.name.clone());

			li().t(name)
				.c(button()
					.on("click", move |_| {
						let dialog_el = dialog_ref
							.get::<DomNode>()
							.unchecked_into::<HtmlDialogElement>();
						managed_library_sig.modify().replace(library.clone());
						dialog_el.show_modal().unwrap();
					})
					.t("Manage"))
				.view(cx)
		})
		.build();

	ul().c(Indexed(cx, props)).view(cx)
}

#[component]
fn LibraryFormFields<'a, G: Html>(
	cx: Scope<'a>,
	library: &'a ReadSignal<Option<LibraryConfig>>,
) -> View<G> {
	use aedron_patchouli_common::libraries::LibraryKind;
	use sycamore::builder::prelude::*;
	use web_sys::{Event, HtmlInputElement};

	let paths_sig = create_signal(
		cx,
		library
			.get()
			.as_ref()
			.as_ref()
			.map(|library| (0..=library.paths.len()).collect())
			.unwrap_or_else(|| vec![0]),
	);
	let indexed_props = IndexedProps::builder()
		.iterable(paths_sig)
		.view(move |cx, index| {
			View::empty()
			/* let value = create_signal(
				cx,
				library
					.get()
					.as_ref()
					.as_ref()
					.map(|library| library.paths.get(index).cloned().unwrap_or_default())
					.unwrap_or_default(),
			);

			input()
				.bind_value(value)
				.dyn_attr("name", || (!value.get().is_empty()).then_some("paths"))
				.dyn_bool_attr("required", move || {
					index.eq(paths_sig.get().first().unwrap())
				})
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
				.view(cx) */
		})
		.build();

	fragment([
		label()
			.t("Name")
			.c(input()
				.attr("name", "name")
				.bool_attr("required", true)
				.dyn_attr("value", || {
					library
						.get()
						.as_ref()
						.as_ref()
						.map(|library| library.name.clone())
				}))
			.view(cx),
		label()
			.t("Type")
			.c(select()
				.attr("name", "kind")
				.bool_attr("required", true)
				.dyn_bool_attr("disabled", || library.get().is_some())
				.c(option().attr("value", "").t("— Please select a type —"))
				.c(View::new_fragment(
					enum_iterator::all::<LibraryKind>()
						.map(|var| {
							let var_s = create_ref(cx, format!("{var:?}"));

							option()
								.attr("value", var_s)
								.dyn_bool_attr("selected", move || {
									library
										.get()
										.as_ref()
										.as_ref()
										.map(|library| library.kind == var)
										.unwrap_or_default()
								})
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
