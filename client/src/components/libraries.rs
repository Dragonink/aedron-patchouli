use aedron_patchouli_common::libraries::PartialLibrary;
use sycamore::{component::Prop, prelude::*};

#[component]
pub(super) fn Libraries<G: Html>(cx: Scope) -> View<G> {
	use sycamore::{
		builder::prelude::*,
		component::Children,
		suspense::{Suspense, SuspenseProps},
	};

	let libraries = create_signal(cx, Vec::<PartialLibrary>::new());
	let props = SuspenseProps::builder()
		.children(Children::new(cx, move |cx| FetchedLibraries(cx, libraries)))
		.fallback(t("Loading..."))
		.build();

	main().c(Suspense(cx, props)).view(cx)
}

#[component]
async fn FetchedLibraries<'a, G: Html>(
	cx: Scope<'a>,
	libraries: &'a Signal<Vec<PartialLibrary>>,
) -> View<G> {
	use aedron_patchouli_common::libraries::API_ENDPOINT;
	use sycamore::builder::prelude::*;
	use web_sys::Request;

	let req = Request::new_with_str(API_ENDPOINT).unwrap();
	libraries.set(crate::fetch_api(&req).await.unwrap().unwrap());
	let props = IndexedProps::builder()
		.iterable(libraries)
		.view(move |cx, library| {
			let name = create_ref(cx, library.name.clone());

			li().c(a().attr("href", format!("/library/{}", library.id)).t(name))
				.view(cx)
		})
		.build();

	ul().c(Indexed(cx, props)).view(cx)
}
