#![forbid(unsafe_code)]
#![deny(unused_must_use)]

use aedron_patchouli_common::users::UserCookie;
use serde::de::DeserializeOwned;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::Request;
use wee_alloc::WeeAlloc;

#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

mod components;
mod router;

fn extract_user_info(cookies: &str) -> Option<UserCookie> {
	cookies
		.split(';')
		.find_map(|s| {
			let s = s.trim();
			s.starts_with(&format!("{}=", UserCookie::COOKIE_NAME))
				.then(move || s.split('=').last().unwrap_throw().to_string())
		})
		.and_then(|uri_enc| {
			let json: String = js_sys::decode_uri_component(&uri_enc).unwrap_throw().into();
			serde_json::from_str(&json)
				.map_err(|err| {
					web_sys::console::error_1(&err.to_string().into());
				})
				.ok()
		})
}

#[wasm_bindgen(start)]
pub fn start() {
	use components::App;
	use web_sys::HtmlDocument;

	std::panic::set_hook(Box::new(console_error_panic_hook::hook));

	sycamore::render(|cx| {
		let doc: HtmlDocument = web_sys::window()
			.and_then(|window| window.document())
			.unwrap_throw()
			.unchecked_into();
		let user_info = extract_user_info(&doc.cookie().unwrap_throw()).unwrap_throw();
		sycamore::reactive::provide_context(cx, user_info);

		App(cx)
	});
}

async fn send_api(req: &Request) -> Result<u16, JsValue> {
	use wasm_bindgen_futures::JsFuture;
	use web_sys::Response;

	let res: Response = JsFuture::from(web_sys::window().unwrap_throw().fetch_with_request(req))
		.await?
		.unchecked_into();
	Ok(res.status())
}

async fn fetch_api<T: DeserializeOwned>(req: &Request) -> Result<Result<T, u16>, JsValue> {
	use js_sys::Uint8Array;
	use wasm_bindgen_futures::JsFuture;
	use web_sys::Response;

	req.headers()
		.set("accept", "application/msgpack")
		.unwrap_throw();
	let res: Response = JsFuture::from(web_sys::window().unwrap_throw().fetch_with_request(req))
		.await?
		.unchecked_into();
	if res.ok() {
		let buf = JsFuture::from(res.array_buffer().unwrap_throw()).await?;
		let mp = Uint8Array::new(&buf).to_vec();
		Ok(Ok(rmp_serde::from_slice(&mp)
			.map_err(|err| web_sys::console::error_1(&err.to_string().into()))
			.unwrap_throw()))
	} else {
		Ok(Err(res.status()))
	}
}
