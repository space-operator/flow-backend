# Squash Baseline (Fresh DB Only)

This folder contains a single baseline migration for provisioning a **brand-new** database.

- Baseline file: `migrations/20260221000000_baseline_v2.sql`
- Source: concatenation of all files in `docker/supabase/migrations/*.sql` in lexical order.
- Existing migration files in `docker/supabase/migrations` were left untouched.

## When to use this

Use this baseline only when the target database has no schema history yet.

Do **not** use this baseline on an existing environment that already applied the old migration chain unless you reset that DB first.

## Why this exists

It gives you a compact starting point for new environments while preserving the original migration history in this repo.

## How to run with flow-server

`crates/flow-server/entrypoint.bash` runs:

```bash
supabase migration up --db-url $MIGRATION_DB_URL
```

Supabase CLI expects a `supabase/migrations` directory in the current working tree (`/space-operator/supabase/migrations` in the container).

### Important path note

Current compose mounts:

```yaml
- ./docker/supabase:/space-operator/supabase:ro
```

To use this squash baseline, temporarily change the mount to:

1. `./docker/supabase/squash:/space-operator/supabase:ro` (use squash baseline), or
2. keep `./docker/supabase:/space-operator/supabase:ro` (use full migration chain).

## Regenerate the baseline

From repo root:

```bash
mkdir -p docker/supabase/squash/migrations
{
  echo "-- Baseline squash for brand-new databases only"
  echo "-- Source: docker/supabase/migrations/*.sql (concatenated in lexical order)"
  for file in $(ls -1 docker/supabase/migrations/*.sql | sort); do
    name=$(basename "$file")
    printf "\n\n-- >>> BEGIN %s\n" "$name"
    cat "$file"
    printf "\n-- <<< END %s\n" "$name"
  done
} > docker/supabase/squash/migrations/20260221000000_baseline_v2.sql
```
