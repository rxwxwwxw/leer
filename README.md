# Local development

Run `make help` to see the available commands. The defaults use `TEST3.jsonl`, `stories.db`, and `users.db` in the current directory. Override them on the command line, for example:

```sh
make import INPUT=/path/to/stories.jsonl STORIES_DB=/path/to/stories.db
make dev STORIES_DB=/path/to/stories.db USERS_DB=/path/to/users.db BASE_PATH=/device2 PORT=8080
```

`make import` requires a database with no stories. `make rebuild-stories` imports into a temporary database beside the destination and replaces it only after the import succeeds. Stop the server and any other process using that database first; the rebuild refuses to proceed if SQLite WAL or shared-memory sidecars are present. The importer builds the search indexes and sampling keys and runs SQLite statistics collection. Production stories databases built with an older schema need to be rebuilt; the separate users database is unaffected.

`make remove-stories` deletes the selected stories database and its SQLite sidecars. `make remove-users` separately deletes the selected users database, including accounts and reading lists. Stop processes using a database before either removal. Neither command is run by a build or test target.

`make dev` runs directly without Caddy at `http://127.0.0.1:8080/device2/` by default; set `BASE_PATH` to test another prefix. `make dev-root` serves at `http://127.0.0.1:8080/`. Both explicitly use development settings and the Makefile database paths. `make run` runs the release server without overriding the environment or `.env`, preserving the existing deployment configuration. `make build` and `make test` build release binaries and run tests, respectively.

Search and tag/keyword browsing expose at most 1,000 stories, in pages of 100. Text-search counts describe the bounded lexical candidate set, not necessarily every match in the corpus. Category pages use a seeded sample of an indexed pseudorandom ordering; the seed in pagination links keeps the sample stable.

Story browsing requires a registered account. Login and its stylesheet remain accessible without a session; public registration is disabled. For production, set `APP_ENV=production` and a persistent, secret `SESSION_SECRET_KEY` of at least 64 bytes; changing the key signs out existing sessions.

## Administrator and user accounts

For a fresh users database, set `ADMIN_USERNAME` and `ADMIN_PASSWORD` in the server environment or an untracked `.env.local` file before starting the application. The password must be at least eight bytes and is stored as an Argon2 hash. `.env.local` takes precedence over `.env`; existing process environment variables take precedence over both. Both `.env` and `.env.local` are ignored by Git. Protect configuration files containing credentials and use HTTPS in production.

The configured account is created only when there are no administrators. If its username already belongs to a non-admin account, startup fails rather than taking it over. For an existing account, explicitly promote it with `cargo run --bin admin_user -- promote USERNAME --database users.db`; use `demote` to revoke admin rights. Once an admin exists, changing the bootstrap environment variables does not change any existing account or password.

After signing in, an administrator can open **My Lists → Manage users** and create accounts individually or in bulk. Bulk input uses one `username,password` pair per line, up to 50 accounts; a comma in a password is allowed. The entire batch fails if any account is invalid or already exists. Passwords are not shown again after submission.
