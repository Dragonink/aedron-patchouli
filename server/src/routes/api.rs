use rocket::{Build, Rocket};

mod libraries;
mod media;
mod users;

#[must_use = "`Rocket<Build>` must be used"]
#[inline]
pub(super) fn mount(rocket: Rocket<Build>) -> Rocket<Build> {
	use aedron_patchouli_common::{libraries, media};

	rocket
		.mount(libraries::API_ENDPOINT, self::libraries::routes())
		.mount(media::API_ENDPOINT, self::media::routes())
}
