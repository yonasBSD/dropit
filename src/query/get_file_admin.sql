SELECT id, admin
FROM files
WHERE short_alias = ? OR long_alias = ?;