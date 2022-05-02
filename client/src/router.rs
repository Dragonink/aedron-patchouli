use sycamore_router::Route as IRoute;

#[derive(Debug, Clone, PartialEq, Eq, IRoute)]
pub(crate) enum Route {
	#[to("/")]
	Home,
	#[to("/library/<id>")]
	Library(u64),
	#[to("/media/<library>/<media>")]
	Media { library: u64, media: u64 },
	#[not_found]
	NotFound,
}
impl Default for Route {
	#[inline(always)]
	fn default() -> Self {
		Self::Home
	}
}
