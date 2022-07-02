use enum_iterator::Sequence;
use std::fmt::{self, Display, Formatter};
use sycamore_router::Route as IRoute;

#[derive(Debug, Clone, PartialEq, Eq, IRoute)]
#[non_exhaustive]
pub(crate) enum Route {
	#[to("/")]
	Home,
	#[to("/dashboard/<_..>")]
	Dashboard(DashboardRoute),
	#[to("/library/<id>")]
	Library { id: u64 },
	#[to("/media/<library>/<media>")]
	Media { library: u64, media: u64 },
	#[not_found]
	NotFound,
}
impl Display for Route {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"/{}",
			match self {
				Self::Home => "".to_string(),
				Self::Dashboard(route) => format!("dashboard{route}"),
				Self::Library { id } => format!("library/{id}"),
				Self::Media { library, media } => format!("media/{library}/{media}"),
				Self::NotFound => "404".to_string(),
			}
		)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Sequence, IRoute)]
#[non_exhaustive]
pub(crate) enum DashboardRoute {
	#[to("/")]
	Profile,
	#[to("/users")]
	Users,
	#[not_found]
	NotFound,
}
impl Display for DashboardRoute {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"/{}",
			match self {
				Self::Profile => "",
				Self::Users => "users",
				Self::NotFound => "404",
			}
		)
	}
}
