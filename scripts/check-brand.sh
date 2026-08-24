#!/usr/bin/env bash
# The b10x string is banned at the surface of this repository. Allowed:
# - Dead github.com/b10x/... URLs ONLY where they are registered or pinned
#   source identity, never documentation (2026-08-24: every such link in AGENTS.md,
#   docs/design/, docs/VISION.md and docs/stories/README.md was dropped, and the
#   Cargo `repository`/`homepage` fields now name github.com/beyond10x/connectors).
#   What remains, and only in these four files:
#   * SOURCES.toml — the human twin of the specs/*.provenance.toml reference lists;
#     the two carry the same bytes and only move together;
#   * crates/catalog-build/src/document.rs — the catalog document `$id`/`$schema`,
#     which is embedded in all 65 catalog/*.catalog.json and in catalog.pack, every
#     one of them sha256-pinned in connectors.lock;
#   * crates/connector-spec/schema/provider-toml.schema.json — the same, for the
#     provider-file schema `$id`;
#   * crates/integration-platform/src/tests.rs — a platform workspaces response
#     fixture, which is that service's wire bytes and not ours to edit.
#   The phrase "the b10x monorepo" survives only in SOURCES.toml's note.
# - Story files (docs/stories/S-*.md): records of what happened, like a
#   changelog — this repository keeps its history there.
# - The b10x-bot GitHub App machinery (scripts/as-bot.sh, bot-token.sh,
#   bot-gh.sh, check-bot-files.py) and the b10x-bot identity in prose:
#   the App's name and its B10X_BOT_* env vars rename only with the App.
# - The published identity of the b10x PLATFORM this repository connects
#   to (D5, resolved 2026-08-23: the platform's NAME is "platform" — crate,
#   type, function and variant names and prose all say platform now — while
#   its published ids stay b10x, per "Identity is irreversible"):
#   * providers/b10x.toml and specs/b10x/ — the provider's public
#     identity and its registered source bytes;
#   * catalog/, catalog.pack and connectors.lock — generated artifacts derived
#     from that identity; regenerating to rename identity is forbidden;
#   * contracts/ and fixtures/ — pinned contract bytes and wire vectors;
#   * scripts/vendor-* — their b10x prose is the pinned provenance bytes
#     they emit into specs/;
#   * the `b10x` provider/initiator id, `io.b10x` authority,
#     `urn:b10x:*` audiences, `b10x.*.vN` contract/profile ids,
#     `b10x/...` hash domains, `x-b10x-*` wire keys, the
#     `b10x.<word>` datasource/channel/state keys, the
#     `workspaces.b10x.io` API group, and the `b10x-owned-contracts`
#     source registry id.
# - The pre-rename serialized surface an existing configuration may still
#   carry: the `[b10x]` config section and `initiation = "b10x"`
#   parse via serde alias forever, and only the compat tests in
#   connectors-config spell them out.
# - Deployment and user state whose rename would strand real data: the
#   `b10x-connectors` service identity (wire User-Agent, vault
#   mount/role, keyring service attribute, /var/lib and /etc paths, secret
#   path prefixes), `~/.config|.local/state/b10x/connectors` personal
#   config/state, the `b10x-operation-audit.jsonl` state file, platform
#   service hosts (b10x-work/-ontology/-planner/-vault), ArgoCD token
#   ids/descriptions this service wrote into external state, and the
#   B10X_SIP_*/B10X_AUDIO_* operator env interface.
# - This check.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

patterns=(
  # dead-monorepo URLs and the monorepo phrase, scoped to the four files that
  # still carry them as registered or pinned identity (see the header)
  '^(SOURCES\.toml|crates/catalog-build/src/document\.rs|crates/connector-spec/schema/provider-toml\.schema\.json|crates/integration-platform/src/tests\.rs):[0-9]+:.*github\.com/b10x'
  '^SOURCES\.toml:[0-9]+:.*the b10x monorepo'
  # bot machinery
  'b10x[-_]bot'
  # provider authority, audiences, wire ids, hash domains, wire keys
  'io\.b10x'
  'urn:b10x'
  'x-b10x-'
  'b10x\.[a-z0-9_-]+\.v[0-9]'
  'b10x(/[a-z-]+)+/v[0-9]'
  'workspaces\.b10x\.io'
  '["'\''`]b10x\.[a-z-]+["'\''`]'
  'b10x-owned-contracts'
  # pre-rename serialized config spellings, alias-parsed forever; the section
  # literal may appear only in the connectors-config compat tests
  'crates/connectors-config/src/(personal|hosted)\.rs:[0-9]+:.*(\[b10x[].]|old_b10x_section)'
  'alias = "b10x"'
  # service identity and deployment/state values
  'b10x-connectors(["/'\''`]|$)'
  'b10x-connectors-sip'
  'b10x-(work|ontology|planner|vault|module-auth|voice|browser|local-operation)'
  '/b10x/(browser|connectors)'
  '"b10x/connectors(\.toml)?"'
  'identity\.dev\.b10x\.example'
  'b10x-operation-audit'
  # provider id grammar: quoted ids, ref/uri grammar, registered artifact
  # paths and file names
  '["'\''`]b10x["'\''`]'
  'b10x(:[^:]|@)'
  '[^:]:b10x([^_a-z]|$)'
  '(specs|providers|catalog)/b10x'
  'b10x\.(provenance\.)?toml'
  'b10x\.catalog\.json'
  'b10x/modules'
  'roles/b10x/'
  '"b10x-b10x"'
  'b10x\?x=1'
  '"b10x/(zwirn|substrate)"'
  'b10x-\{'
  '"description": "b10x connector"'
  '\.b10x/dev\.local\.json'
  # the platform-modules design doc records its own D5 retitle
  'retitled from "b10x module admission"'
  # operator env interface of the native drivers
  'b10x_(sip|audio)_'
)
allow=$(IFS='|'; printf '%s' "${patterns[*]}")

# Negative self-test: what S-052 renamed must never come back. Each sample is
# a violation the allowlist must NOT swallow; if one slips through, the fence
# is broken and this check fails before it checks anything.
negatives=(
  'crates/x/Cargo.toml:2:name = "integration-b10x"'
  'crates/x/src/lib.rs:1:use integration_b10x::PlatformBackend;'
  'crates/x/src/lib.rs:2:pub struct B10xBackend;'
  'crates/x/src/lib.rs:3:pub struct B10xIntegrationConfig;'
  'crates/x/src/lib.rs:4:let policy = InitiationPolicy::b10x_only();'
  'crates/x/src/lib.rs:5:ConnectionInitiator::B10x'
  'docs/design/x.md:1:the B10x Provider signs module requests'
  'docs/design/x.md:2:the B10x Integration and the B10x Identity verifier'
  'crates/x/src/backend.rs:1:<p>Authorize B10x to use GitLab.</p>'
)
for sample in "${negatives[@]}"; do
  if ! printf '%s\n' "$sample" | grep -qviE "$allow"; then
    printf 'brand check: self-test failed, the allowlist swallows:\n%s\n' "$sample" >&2
    exit 1
  fi
done

hits=$(git grep -in 'b10x' -- \
  ':!specs' ':!providers' ':!catalog' ':!contracts' ':!fixtures' \
  ':!connectors.lock' ':!crates/catalog-reader/catalog.pack' \
  ':!docs/stories/S-*.md' \
  ':!scripts/as-bot.sh' ':!scripts/bot-token.sh' ':!scripts/bot-gh.sh' \
  ':!scripts/check-bot-files.py' ':!scripts/vendor-*' \
  ':!scripts/check-brand.sh' \
  | grep -viE "$allow" || true)
if test -n "$hits"; then
  printf 'brand check: b10x at the surface:\n%s\n' "$hits" >&2
  exit 1
fi
printf 'brand check: clean\n'
