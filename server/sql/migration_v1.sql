BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE libraries (
	id INTEGER NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	kind INTEGER NOT NULL CHECK (kind >= 0 AND kind <= 1),
	paths TEXT NOT NULL
) STRICT;

CREATE TABLE media_image (
	id INTEGER NOT NULL PRIMARY KEY,
	library INTEGER NOT NULL REFERENCES libraries (id) ON DELETE CASCADE,
	path TEXT NOT NULL,
	title TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX media_image_fs ON media_image (
	library,
	path
);
CREATE TRIGGER check_library_kind_on_insert_image
BEFORE INSERT ON media_image
BEGIN
	SELECT RAISE(FAIL, "expected library kind to be Image")
	FROM libraries
	WHERE id = NEW.library AND kind != 0;
END;
CREATE TRIGGER check_library_kind_on_update_image
BEFORE UPDATE OF library ON media_image
BEGIN
	SELECT RAISE(FAIL, "expected library kind to be Image")
	FROM libraries
	WHERE id = NEW.library AND kind != 0;
END;

CREATE TABLE media_music (
	id INTEGER NOT NULL PRIMARY KEY,
	library INTEGER NOT NULL REFERENCES libraries (id) ON DELETE CASCADE,
	path TEXT NOT NULL,
	title TEXT NOT NULL,
	artist TEXT,
	album TEXT,
	track INTEGER
) STRICT;
CREATE UNIQUE INDEX media_music_fs ON media_music (
	library,
	path
);
CREATE TRIGGER check_library_kind_on_insert_music
BEFORE INSERT ON media_music
BEGIN
	SELECT RAISE(FAIL, "expected library kind to be Music")
	FROM libraries
	WHERE id = NEW.library AND kind != 1;
END;
CREATE TRIGGER check_library_kind_on_update_music
BEFORE UPDATE OF library ON media_music
BEGIN
	SELECT RAISE(FAIL, "expected library kind to be Music")
	FROM libraries
	WHERE id = NEW.library AND kind != 1;
END;

PRAGMA user_version = 1;
COMMIT TRANSACTION;
