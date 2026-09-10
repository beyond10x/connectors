CREATE TABLE local_runtime_instances (
    instance_id TEXT PRIMARY KEY,
    suppressed INTEGER NOT NULL DEFAULT 0 CHECK(suppressed IN (0,1)),
    selection TEXT,
    bootstrap TEXT,
    observed_at_ms INTEGER,
    CHECK((selection IS NULL AND bootstrap IS NULL AND observed_at_ms IS NULL) OR
          (selection IS NOT NULL AND bootstrap IS NOT NULL AND observed_at_ms >= 0))
) STRICT;
