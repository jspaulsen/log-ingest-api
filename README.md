# Log Ingest API

API for ingesting log messages

## Database Migrations

`log-ingest-api` leverages `sqlx-cli` (and `sqlx` in code) for database migrations.

```bash
# It doesn't run against a live database (afaik), so you can use any database url
DATABASE_URL=postgres://postgres@localhost/my_database sqlx migrate add <name>
```

## Development

### Database Tests

Database tests by default are ignored.  To run database test, start the docker-compose postgres instance:

```bash
docker-compose -f docker-compose.test.yaml up -d
```

Then run the tests with the `DATABASE_URL` environment variable set:

```bash
cargo test -- --include-ignored --test-threads=1
```
