ALTER TABLE mutation_attempts ADD COLUMN approval_subject TEXT
  CHECK(approval_subject IS NULL OR length(CAST(approval_subject AS BLOB)) BETWEEN 1 AND 8192);
CREATE TABLE approval_redemptions (
  receipt_id TEXT PRIMARY KEY,
  issuer TEXT NOT NULL CHECK(length(CAST(issuer AS BLOB)) BETWEEN 1 AND 256),
  reference TEXT NOT NULL CHECK(length(CAST(reference AS BLOB))=64),
  instance_id TEXT NOT NULL REFERENCES registry_instances(instance_id),
  attempt_id TEXT NOT NULL UNIQUE REFERENCES mutation_attempts(attempt_id),
  subject TEXT NOT NULL CHECK(length(CAST(subject AS BLOB)) BETWEEN 1 AND 8192),
  spent_at_ms INTEGER NOT NULL CHECK(spent_at_ms>=0),
  UNIQUE(issuer,reference)
) STRICT;
CREATE INDEX approval_redemption_instance ON approval_redemptions(instance_id);
CREATE TRIGGER approval_redemption_immutable_update BEFORE UPDATE ON approval_redemptions
BEGIN SELECT RAISE(ABORT,'immutable approval redemption'); END;
CREATE TRIGGER approval_redemption_immutable_delete BEFORE DELETE ON approval_redemptions
BEGIN SELECT RAISE(ABORT,'immutable approval redemption'); END;
