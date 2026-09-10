CREATE TABLE execution_audits (
  audit_record_ref TEXT PRIMARY KEY CHECK(length(CAST(audit_record_ref AS BLOB))<=512),
  instance_id TEXT NOT NULL REFERENCES registry_instances(instance_id),
  audit_ref TEXT NOT NULL CHECK(length(CAST(audit_ref AS BLOB)) BETWEEN 1 AND 128),
  connection_ref TEXT REFERENCES registry_connections(connection_ref),
  attempt_id TEXT REFERENCES mutation_attempts(attempt_id),
  record_json TEXT NOT NULL,
  UNIQUE(instance_id,audit_ref),
  CHECK(length(CAST(record_json AS BLOB))+length(CAST(audit_record_ref AS BLOB))<=4096)
) STRICT;
CREATE INDEX execution_audit_instance ON execution_audits(instance_id);
