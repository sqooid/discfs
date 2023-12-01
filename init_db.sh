rm target/dev.db
script="$(cat src/local/create_schema.sql)"
sqlite3 target/dev.db "$script"