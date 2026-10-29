.DEFAULT_GOAL := help

CARGO ?= cargo
INPUT ?= TEST3.jsonl
STORIES_DB ?= stories.db
USERS_DB ?= users.db
BASE_PATH ?= /device2
HOST ?= 127.0.0.1
PORT ?= 8080

.PHONY: help build test import rebuild-stories remove-stories remove-users dev dev-root run

help:
	@printf '%s\n' \
	  'make build             Build the release binaries' \
	  'make test              Run the test suite' \
	  'make import            Import INPUT into an empty STORIES_DB' \
	  'make rebuild-stories   Replace STORIES_DB after a successful import' \
	  'make remove-stories    Delete STORIES_DB and its SQLite sidecars' \
	  'make remove-users      Delete USERS_DB and its SQLite sidecars, including accounts and lists' \
	  'make dev               Run locally at BASE_PATH (default /device2)' \
	  'make dev-root          Run locally at /' \
	  'make run               Run the release server with its normal environment and .env' \
	  '' \
	  'Overrides: INPUT, STORIES_DB, USERS_DB, BASE_PATH, HOST, PORT, CARGO'

build:
	$(CARGO) build --release --bins

test:
	$(CARGO) test

import:
	$(CARGO) run --release --bin load_data -- "$(INPUT)" "$(STORIES_DB)"

# Build beside the destination so the final rename is atomic and a failed import
# leaves the previous database in place. Stop all servers using it first.
rebuild-stories:
	@set -eu; \
	  db='$(STORIES_DB)'; \
	  if [ -e "$$db-wal" ] || [ -e "$$db-shm" ]; then \
	    printf '%s\n' 'SQLite sidecars exist; stop all processes using the stories database and check its WAL before rebuilding.' >&2; exit 1; \
	  fi; \
	  tmp=$$(mktemp "$${db}.tmp.XXXXXX"); \
	  trap 'rm -f -- "$$tmp" "$$tmp-wal" "$$tmp-shm"' EXIT HUP INT TERM; \
	  $(CARGO) run --release --bin load_data -- "$(INPUT)" "$$tmp"; \
	  mv -f -- "$$tmp" "$$db"; \
	  printf 'Rebuilt %s\n' "$$db"

# These explicit targets delete data; never make them prerequisites of other targets.
remove-stories:
	rm -f -- "$(STORIES_DB)" "$(STORIES_DB)-wal" "$(STORIES_DB)-shm"

remove-users:
	rm -f -- "$(USERS_DB)" "$(USERS_DB)-wal" "$(USERS_DB)-shm"

dev:
	APP_ENV=development BASE_PATH="$(BASE_PATH)" HOST="$(HOST)" PORT="$(PORT)" STORIES_DB_URL="$(STORIES_DB)" USERS_DB_URL="$(USERS_DB)" $(CARGO) run --bin NG-Reader

dev-root:
	APP_ENV=development BASE_PATH= HOST="$(HOST)" PORT="$(PORT)" STORIES_DB_URL="$(STORIES_DB)" USERS_DB_URL="$(USERS_DB)" $(CARGO) run --bin NG-Reader

# Do not override deployment configuration supplied by the environment or .env.
run:
	$(CARGO) run --release --bin NG-Reader
