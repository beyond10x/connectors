CREATE TABLE mutation_clock (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1),
  last_lower_ms INTEGER NOT NULL CHECK(last_lower_ms>=0)
) STRICT;
INSERT INTO mutation_clock VALUES (1,0);
CREATE TABLE mutation_attempts (
  attempt_id TEXT PRIMARY KEY,
  instance_id TEXT NOT NULL REFERENCES registry_instances(instance_id),
  connection_ref TEXT NOT NULL REFERENCES registry_connections(connection_ref),
  request_id TEXT NOT NULL CHECK(length(CAST(request_id AS BLOB)) BETWEEN 1 AND 256),
  fingerprint TEXT NOT NULL CHECK(length(CAST(fingerprint AS BLOB))<=16384),
  approval_mode TEXT NOT NULL CHECK(approval_mode IN ('not_required','required','event_claim')),
  approval_ref TEXT CHECK(length(CAST(approval_ref AS BLOB)) BETWEEN 1 AND 256),
  owner_nonce TEXT NOT NULL UNIQUE,
  publication_fence TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('prepared','dispatching','aborted','completed','failed','indeterminate')),
  settled_at_ms INTEGER CHECK(settled_at_ms>=0),
  result_json TEXT CHECK(length(CAST(result_json AS BLOB))<=1048576),
  CHECK((state IN ('aborted','completed','failed'))=(settled_at_ms IS NOT NULL)),
  CHECK((state IN ('aborted','completed','failed','indeterminate'))=(result_json IS NOT NULL)),
  CHECK(approval_mode!='not_required' OR approval_ref IS NULL)
) STRICT;
CREATE INDEX mutation_attempt_instance ON mutation_attempts(instance_id,state);
CREATE TABLE mutation_keys (
  reservation_id TEXT PRIMARY KEY,
  namespace_key TEXT NOT NULL CHECK(length(CAST(namespace_key AS BLOB))<=4096),
  attempt_id TEXT NOT NULL UNIQUE REFERENCES mutation_attempts(attempt_id),
  fingerprint TEXT NOT NULL CHECK(length(CAST(fingerprint AS BLOB))<=16384),
  state TEXT NOT NULL CHECK(state IN ('pending','replayable','quarantined','expired')),
  settled_at_ms INTEGER CHECK(settled_at_ms>=0),
  replay_expires_at_ms INTEGER,
  CHECK((state IN ('replayable','expired'))=(settled_at_ms IS NOT NULL)),
  CHECK((settled_at_ms IS NULL)=(replay_expires_at_ms IS NULL)),
  CHECK(replay_expires_at_ms IS NULL OR replay_expires_at_ms=settled_at_ms+86400000)
) STRICT;
CREATE UNIQUE INDEX mutation_live_key ON mutation_keys(namespace_key) WHERE state!='expired';
