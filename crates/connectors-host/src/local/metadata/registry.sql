CREATE TABLE registry_clock (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1), last_seen_ms INTEGER NOT NULL CHECK(last_seen_ms>=0)
) STRICT;
INSERT INTO registry_clock VALUES (1, 0);
CREATE TABLE registry_instances (
  instance_id TEXT PRIMARY KEY, adapter_id TEXT NOT NULL,
  configuration_revision TEXT NOT NULL, epoch INTEGER NOT NULL DEFAULT 0 CHECK(epoch>=0)
) STRICT;
CREATE TABLE registry_profiles (
  profile_key TEXT PRIMARY KEY, adapter_id TEXT NOT NULL, profile_ref TEXT NOT NULL,
  revision TEXT NOT NULL, declaration TEXT NOT NULL CHECK(length(declaration)<=32768),
  UNIQUE(adapter_id, profile_ref, revision)
) STRICT;
CREATE TABLE registry_connections (
  connection_ref TEXT PRIMARY KEY, instance_id TEXT NOT NULL REFERENCES registry_instances(instance_id),
  profile_key TEXT NOT NULL REFERENCES registry_profiles(profile_key),
  binding TEXT NOT NULL CHECK(length(binding)<=32768),
  scope_id TEXT NOT NULL UNIQUE, semantic_revision TEXT NOT NULL, publication_fence TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('live','revoked')), public INTEGER NOT NULL DEFAULT 0 CHECK(public IN (0,1)),
  identity TEXT CHECK(identity IS NULL OR length(identity)<=8192),
  active_generation TEXT REFERENCES registry_generations(generation_id),
  active_material TEXT REFERENCES registry_materials(version_id),
  baseline TEXT CHECK(baseline IS NULL OR length(baseline)<=32768),
  created_at_ms INTEGER NOT NULL CHECK(created_at_ms>0), revoked_at_ms INTEGER,
  CHECK((active_generation IS NULL)=(active_material IS NULL)),
  CHECK((active_generation IS NULL)=(baseline IS NULL)),
  CHECK(state!='revoked' OR (active_material IS NULL AND revoked_at_ms IS NOT NULL))
) STRICT;
CREATE INDEX registry_connection_instance ON registry_connections(instance_id, public, connection_ref);
CREATE TABLE registry_acquisitions (
  acquisition_ref TEXT PRIMARY KEY, connection_ref TEXT NOT NULL REFERENCES registry_connections(connection_ref),
  owner_token TEXT NOT NULL UNIQUE, expected_fence TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('pending','completing','completed','failed','expired')),
  failure TEXT CHECK(failure IN ('rejected','exchange_unknown','expired')),
  created_at_ms INTEGER NOT NULL, expires_at_ms INTEGER NOT NULL CHECK(expires_at_ms>created_at_ms),
  consumed_at_ms INTEGER,
  generation_id TEXT NOT NULL UNIQUE, capture_id TEXT NOT NULL UNIQUE,
  candidate_id TEXT UNIQUE REFERENCES registry_materials(version_id),
  CHECK((state IN ('failed','expired'))=(failure IS NOT NULL))
) STRICT;
CREATE INDEX registry_acquisition_expiry ON registry_acquisitions(state, expires_at_ms);
CREATE TABLE registry_generations (
  generation_id TEXT PRIMARY KEY, connection_ref TEXT NOT NULL REFERENCES registry_connections(connection_ref),
  capture_id TEXT NOT NULL UNIQUE, expected_identity TEXT NOT NULL CHECK(length(expected_identity)<=8192)
) STRICT;
CREATE TABLE registry_materials (
  version_id TEXT PRIMARY KEY, connection_ref TEXT NOT NULL REFERENCES registry_connections(connection_ref),
  acquisition_ref TEXT NOT NULL UNIQUE REFERENCES registry_acquisitions(acquisition_ref),
  generation_id TEXT NOT NULL UNIQUE REFERENCES registry_generations(generation_id),
  acknowledged INTEGER NOT NULL DEFAULT 0 CHECK(acknowledged IN (0,1)),
  acknowledged_at_ms INTEGER,
  deleted INTEGER NOT NULL DEFAULT 0 CHECK(deleted IN (0,1)),
  invalid_reason TEXT CHECK(invalid_reason IN ('missing','invalid','revoked','insufficient','uncertain')),
  byte_size INTEGER CHECK(byte_size BETWEEN 1 AND 65536),
  retirement_fence TEXT, retired_at_ms INTEGER, delete_not_before_ms INTEGER,
  CHECK((retirement_fence IS NULL)=(retired_at_ms IS NULL)),
  CHECK((acknowledged=1)=(acknowledged_at_ms IS NOT NULL)),
  CHECK((retirement_fence IS NULL)=(delete_not_before_ms IS NULL)),
  CHECK(deleted=0 OR (retirement_fence IS NOT NULL AND byte_size IS NULL))
) STRICT;
CREATE INDEX registry_material_connection ON registry_materials(connection_ref, deleted);
CREATE TABLE registry_uses (
  use_id TEXT PRIMARY KEY, connection_ref TEXT NOT NULL REFERENCES registry_connections(connection_ref),
  generation_id TEXT NOT NULL REFERENCES registry_generations(generation_id),
  version_id TEXT NOT NULL REFERENCES registry_materials(version_id),
  publication_fence TEXT NOT NULL, expires_at_ms INTEGER NOT NULL,
  dispatched INTEGER NOT NULL DEFAULT 0 CHECK(dispatched IN (0,1)),
  released INTEGER NOT NULL DEFAULT 0 CHECK(released IN (0,1))
) STRICT;
CREATE INDEX registry_use_retention ON registry_uses(version_id, released, expires_at_ms);
CREATE TABLE registry_cursors (
  cursor_id TEXT PRIMARY KEY, instance_id TEXT NOT NULL REFERENCES registry_instances(instance_id),
  adapter_id TEXT NOT NULL, configuration_revision TEXT NOT NULL, epoch INTEGER NOT NULL,
  last_connection TEXT NOT NULL, page_limit INTEGER NOT NULL CHECK(page_limit BETWEEN 1 AND 500),
  expires_at_ms INTEGER NOT NULL
) STRICT;
