#!/usr/bin/env bash
set -Eeuo pipefail
ls migrations -l
supabase migration up --db-url $MIGRATION_DB_URL
unset MIGRATION_DB_URL
./flow-server $CONFIG_FILE