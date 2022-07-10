BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE users (
	id INTEGER NOT NULL PRIMARY KEY,
	name TEXT NOT NULL UNIQUE,
	passwd TEXT NOT NULL
) STRICT;

CREATE TABLE permissions (
	library INTEGER NOT NULL REFERENCES libraries (id) ON DELETE CASCADE,
	user INTEGER REFERENCES users (id) ON DELETE CASCADE CHECK(user != 1),
	action INTEGER CHECK(action = -1 OR action = 1),

	CHECK(user IS NOT NULL OR action IS NOT NULL)
) STRICT;
CREATE UNIQUE INDEX permission_scope ON permissions (
	library,
	user
);
CREATE VIEW effect_permissions AS
SELECT library, user, CASE action
	WHEN NULL THEN (SELECT action FROM permissions as p WHERE p.library = permissions.library AND user IS NULL)
	ELSE action
END as action FROM permissions;

PRAGMA user_version = 2;
COMMIT TRANSACTION;
