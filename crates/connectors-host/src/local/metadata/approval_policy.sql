CREATE TABLE local_approval_policies (
  policy_id TEXT PRIMARY KEY,
  instance_id TEXT NOT NULL UNIQUE REFERENCES registry_instances(instance_id),
  issuer_id TEXT NOT NULL REFERENCES local_approval_issuers(issuer_id),
  revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 9007199254740991),
  owner_uid INTEGER NOT NULL CHECK(owner_uid BETWEEN 0 AND 4294967295),
  selection TEXT NOT NULL CHECK(length(selection) <= 4096),
  operations TEXT NOT NULL CHECK(length(operations) <= 131072)
) STRICT;
CREATE TRIGGER local_approval_policy_identity BEFORE UPDATE ON local_approval_policies
WHEN OLD.policy_id IS NOT NEW.policy_id OR OLD.instance_id IS NOT NEW.instance_id
  OR OLD.issuer_id IS NOT NEW.issuer_id OR OLD.owner_uid IS NOT NEW.owner_uid
BEGIN SELECT RAISE(ABORT,'immutable policy identity'); END;
CREATE TRIGGER local_approval_policy_retained BEFORE DELETE ON local_approval_policies
BEGIN SELECT RAISE(ABORT,'retained approval policy'); END;
CREATE TRIGGER local_approval_policy_revision BEFORE UPDATE ON local_approval_policies
WHEN NEW.revision != OLD.revision + 1
BEGIN SELECT RAISE(ABORT,'non-successor policy revision'); END;
CREATE TRIGGER local_approval_policy_issuer BEFORE INSERT ON local_approval_policies
WHEN NOT EXISTS (SELECT 1 FROM local_approval_issuers
  WHERE issuer_id=NEW.issuer_id AND instance_id=NEW.instance_id)
BEGIN SELECT RAISE(ABORT,'policy issuer instance mismatch'); END;
