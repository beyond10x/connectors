CREATE TABLE local_approval_issuers (
  issuer_id TEXT PRIMARY KEY,
  instance_id TEXT NOT NULL UNIQUE REFERENCES registry_instances(instance_id),
  revision TEXT NOT NULL UNIQUE,
  custody_scope TEXT NOT NULL UNIQUE
) STRICT;
CREATE TABLE local_approval_keys (
  key_id TEXT PRIMARY KEY,
  issuer_id TEXT NOT NULL REFERENCES local_approval_issuers(issuer_id),
  public_key TEXT NOT NULL CHECK(length(public_key)=43),
  material_version TEXT NOT NULL UNIQUE,
  state TEXT NOT NULL CHECK(state IN ('candidate','active','retired','retiring','deleted'))
) STRICT;
CREATE UNIQUE INDEX local_approval_active ON local_approval_keys(issuer_id) WHERE state='active';
CREATE UNIQUE INDEX local_approval_candidate ON local_approval_keys(issuer_id) WHERE state='candidate';
CREATE TRIGGER local_approval_issuer_identity BEFORE UPDATE ON local_approval_issuers
WHEN OLD.issuer_id IS NOT NEW.issuer_id OR OLD.instance_id IS NOT NEW.instance_id
  OR OLD.custody_scope IS NOT NEW.custody_scope
BEGIN SELECT RAISE(ABORT,'immutable issuer identity'); END;
CREATE TRIGGER local_approval_issuer_retained BEFORE DELETE ON local_approval_issuers
BEGIN SELECT RAISE(ABORT,'retained issuer'); END;
CREATE TRIGGER local_approval_key_identity BEFORE UPDATE ON local_approval_keys
WHEN OLD.key_id IS NOT NEW.key_id OR OLD.issuer_id IS NOT NEW.issuer_id
  OR OLD.public_key IS NOT NEW.public_key OR OLD.material_version IS NOT NEW.material_version
BEGIN SELECT RAISE(ABORT,'immutable signing key identity'); END;
CREATE TRIGGER local_approval_key_retained BEFORE DELETE ON local_approval_keys
BEGIN SELECT RAISE(ABORT,'retained signing key'); END;
CREATE TRIGGER local_approval_key_transition BEFORE UPDATE OF state ON local_approval_keys
WHEN OLD.state != NEW.state AND NOT (
  (OLD.state='candidate' AND NEW.state IN ('active','retired','retiring')) OR
  (OLD.state='active' AND NEW.state='retired') OR
  (OLD.state='retired' AND NEW.state='retiring') OR
  (OLD.state='retiring' AND NEW.state='deleted'))
BEGIN SELECT RAISE(ABORT,'invalid signing key transition'); END;
CREATE TRIGGER local_approval_key_capacity BEFORE INSERT ON local_approval_keys
WHEN (SELECT count(*) FROM local_approval_keys WHERE issuer_id=NEW.issuer_id)>=128
BEGIN SELECT RAISE(ABORT,'signing key capacity'); END;
