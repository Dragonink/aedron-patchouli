#![forbid(unsafe_code)]
#![deny(unused_must_use)]

use serde::de::DeserializeOwned;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::Request;
use wee_alloc::WeeAlloc;

#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

mod components;
mod router;

#[wasm_bindgen(start)]
pub fn start() {
	use components::App;

	std::panic::set_hook(Box::new(console_error_panic_hook::hook));

	sycamore::render(|cx| App(cx));
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
