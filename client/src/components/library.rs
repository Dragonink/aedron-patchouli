use aedron_patchouli_common::library::Library;
use sycamore::{component::Prop, prelude::*};
use wasm_bindgen::UnwrapThrowExt;

#[component]
pub(super) fn Library<G: Html>(cx: Scope, id: u64) -> View<G> {
	use sycamore::{
		builder::prelude::*,
		suspense::{Suspense, SuspenseProps},
	};

	let props = SuspenseProps::builder()
		.children(Children::new(cx, move |cx| FetchedLibrary(cx, id)))
		.fallback(t("Loading..."))
		.build();

	main().c(Suspense(cx, props)).view(cx)
}

#[component]
async fn FetchedLibrary<G: Html>(cx: Scope<'_>, id: u64) -> View<G> {
	use aedron_patchouli_common::{
		library::{LibraryKind, PartialLibrary, API_ENDPOINT},
		media::{MediaImage, MediaMusic},
	};
	use sycamore::builder::prelude::*;
	use web_sys::Request;

	let mut req = Request::new_with_str(&format!("{API_ENDPOINT}/{id}")).unwrap_throw();
	let library: PartialLibrary = crate::fetch_api(&req).await.unwrap_throw().unwrap_throw();
	let name = create_ref(cx, library.name);

	req = Request::new_with_str(&format!("{API_ENDPOINT}/{id}?full=true")).unwrap_throw();
	let frag = match library.kind {
		LibraryKind::Image => {
			let library: Library<MediaImage> =
				crate::fetch_api(&req).await.unwrap_throw().unwrap_throw();
			View::new_fragment(
				library
					.media
					.into_iter()
					.map(|file| {
						let title = create_ref(cx, file.title);

						li().c(a()
							.attr("href", format!("/media/{}/{}", library.id, file.id))
							.t(title))
							.view(cx)
					})
					.collect(),
			)
		}
		LibraryKind::Music => {
			let library: Library<MediaMusic> =
				crate::fetch_api(&req).await.unwrap_throw().unwrap_throw();
			View::new_fragment(
				library
					.media
					.into_iter()
					.map(|file| {
						let title = create_ref(cx, file.title);

						li().c(a()
							.attr("href", format!("/media/{}/{}", library.id, file.id))
							.t(title))
							.view(cx)
					})
					.collect(),
			)
		}
		_ => todo!(),
	};

	fragment([h1().t(name).view(cx), ul().c(frag).view(cx)])
}
