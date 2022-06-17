use sycamore::{component::Prop, prelude::*};
use wasm_bindgen::UnwrapThrowExt;

#[derive(Prop)]
pub(super) struct MediaProps {
	pub library: u64,
	pub media: u64,
}
#[component]
pub(super) fn Media<G: Html>(cx: Scope, media_props: MediaProps) -> View<G> {
	use sycamore::{
		builder::prelude::*,
		suspense::{Suspense, SuspenseProps},
	};

	let props = SuspenseProps::builder()
		.children(Children::new(cx, |cx| MediaSwitch(cx, media_props)))
		.fallback(t("Loading..."))
		.build();

	main().c(Suspense(cx, props)).view(cx)
}

#[component]
async fn MediaSwitch<G: Html>(cx: Scope<'_>, props: MediaProps) -> View<G> {
	use aedron_patchouli_common::libraries::{LibraryKind, PartialLibrary, API_ENDPOINT};
	use web_sys::Request;

	let req = Request::new_with_str(&format!("{API_ENDPOINT}/{}", props.library)).unwrap_throw();
	let library: PartialLibrary = crate::fetch_api(&req).await.unwrap_throw().unwrap_throw();

	match library.kind {
		LibraryKind::Image => MediaImage(cx, props),
		LibraryKind::Music => MediaMusic(cx, props),
		_ => todo!(),
	}
}

#[component]
async fn MediaImage<G: Html>(cx: Scope<'_>, props: MediaProps) -> View<G> {
	use aedron_patchouli_common::media::{MediaImage, API_ENDPOINT};
	use sycamore::builder::prelude::*;
	use web_sys::Request;

	let MediaProps { library, media } = props;

	let req =
		Request::new_with_str(&format!("{API_ENDPOINT}/{}/{}", library, media)).unwrap_throw();
	let image: MediaImage = crate::fetch_api(&req).await.unwrap_throw().unwrap_throw();
	let title = create_ref(cx, image.title);

	fragment([
		img()
			.attr("src", format!("/media?library={library}&file={media}"))
			.view(cx),
		h1().t(title).view(cx),
	])
}

#[component]
async fn MediaMusic<G: Html>(cx: Scope<'_>, props: MediaProps) -> View<G> {
	use aedron_patchouli_common::media::{MediaMusic, API_ENDPOINT};
	use sycamore::builder::prelude::*;
	use web_sys::Request;

	let MediaProps { library, media } = props;

	let req =
		Request::new_with_str(&format!("{API_ENDPOINT}/{}/{}", library, media)).unwrap_throw();
	let image: MediaMusic = crate::fetch_api(&req).await.unwrap_throw().unwrap_throw();
	let title = create_ref(cx, image.title);

	fragment([
		audio()
			.bool_attr("controls", true)
			.attr("preload", "metadata")
			.attr("src", format!("/media?library={library}&file={media}"))
			.view(cx),
		h1().t(title).view(cx),
	])
}
