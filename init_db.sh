rm fs.db
script="$(cat src/local/create_schema.sql)"
sqlite3 fs.db "$script"