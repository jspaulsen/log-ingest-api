# Log Ingest API

API for ingesting log messages

## Configuration

Configuration is done via environment variables.  The following variables are supported:

`DATABASE_URL` (Required) - The database URL application connects to.  Must be in `postgres://` format.
`HTTP_PORT` (Optional) - The port the HTTP server listens on.  Defaults to `8080`.
`HTTP_HOST` (Optional) - The host the HTTP server listens on.  Defaults to `0.0.0.0`.
`LOG_LEVEL` (Optional) - The log level for the application.  Defaults to `info`.

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
