#!/usr/bin/env bash
# The b10x string is banned at the surface of this repository. Allowed:
# - Pinned provenance URLs (github.com/b10x/...) and the phrase
#   "the b10x monorepo" in extraction-provenance prose.
# - Story files (docs/stories/S-*.md): records of what happened, like a
#   changelog — this repository keeps its history there.
# - The b10x-bot GitHub App machinery (scripts/as-bot.sh, bot-token.sh,
#   bot-gh.sh, check-bot-files.py) and the b10x-bot identity in prose:
#   the App's name and its B10X_BOT_* env vars rename only with the App.
# - The b10x PLATFORM this repository connects to (decision D5 in the
#   2026-08-23 beyond10x extraction plan keeps the platform's name open, so
#   nothing here may pre-empt it):
#   * providers/b10x.toml and specs/b10x/ — the provider's public
#     identity and its registered source bytes ("Identity is irreversible");
#   * catalog/, catalog.pack and connectors.lock — generated artifacts derived
#     from that identity; regenerating to rename identity is forbidden;
#   * contracts/ and fixtures/ — pinned contract bytes and wire vectors;
#   * scripts/vendor-* — their b10x prose is the pinned provenance bytes
#     they emit into specs/;
#   * crates/integration-b10x and the B10x* code names bound to
#     the provider (B10xConnectionConfig et al., S-051);
#   * the `b10x` provider/initiator id, `io.b10x` authority,
#     `urn:b10x:*` audiences, `b10x.*.vN` contract/profile ids,
#     `b10x/...` hash domains, `x-b10x-*` wire keys, and the
#     "B10x Provider/Integration/Identity/..." platform prose (D5).
# - Deployment and user state whose rename would strand real data: the
#   `b10x-connectors` service identity (wire User-Agent, vault
#   mount/role, keyring service attribute, /var/lib and /etc paths, secret
#   path prefixes), `~/.config|.local/state/b10x/connectors` personal
#   config/state, platform service hosts (b10x-work/-ontology/-planner/
#   -vault), ArgoCD token ids/descriptions this service wrote into external
#   state, and the B10X_SIP_*/B10X_AUDIO_* operator env interface.
# - This check.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

patterns=(
  # provenance
  'github\.com/b10x'
  'the b10x monorepo'
  # bot machinery
  'b10x[-_]bot'
  # platform crate (D5)
  'integration-b10x'
  # provider authority, audiences, wire ids, hash domains, wire keys
  'io\.b10x'
  'urn:b10x'
  'x-b10x-'
  'b10x\.[a-z0-9_-]+\.v[0-9]'
  'b10x(/[a-z-]+)+/v[0-9]'
  # service identity and deployment/state values
  'b10x-connectors(["/'\''`]|$)'
  'b10x-connectors-sip'
  'b10x-(work|ontology|planner|vault|module-auth|voice|browser|local-operation)'
  '/b10x/(browser|connectors)'
  '"b10x/connectors(\.toml)?"'
  'identity\.dev\.b10x\.example'
  # provider id grammar: quoted ids, TOML sections, ref/uri grammar,
  # registered artifact paths and file names
  '["'\''`]b10x["'\''`]'
  '\\"b10x\\"'
  '\[b10x[].]'
  'b10x[:@]'
  ':b10x'
  '(specs|providers|catalog)/b10x'
  'b10x\.(provenance\.)?toml'
  'b10x\.catalog\.json'
  'b10x/modules'
  'roles/b10x/'
  '"b10x-b10x"'
  'b10x\?x=1'
  '"b10x/(zwirn|substrate)"'
  'b10x-\{'
  # D5-bound code identifiers and the config field they hang off
  'b10x_(only|enabled|configured|document|dial_member|provider_satisfies)'
  'b10x(connectionconfig|integrationconfig|backend|integrationerror)'
  '::b10x'
  '\.b10x'
  '\|b10x\|'
  'b10x\.(validate|allows)'
  ':[0-9]+:[[:space:]]*b10x,$'
  'catalog-b10x-test'
  'b10x   enum'
  # operator env interface of the native drivers
  'b10x_(sip|audio)_'
  # platform prose (D5): the provider/platform as named actor
  'b10x (provider|integration|identity|session authority|module|principal|caller|capability|catalog|member|declaration|profile|operations|adoption|source|service|private services|connector\b)'
  "b10x's"
  'b10x-owned (read|local|browser|native|connector)'
  'b10x-owned-contracts'
  'permit b10x to initiate'
  'b10x is one allowed'
  'authorize b10x to use'
  'connection b10x makes'
  'b10x or babelforce'
  'toward b10x'
  'b10x `'
)
allow=$(IFS='|'; printf '%s' "${patterns[*]}")

hits=$(git grep -in 'b10x' -- \
  ':!specs' ':!providers' ':!catalog' ':!contracts' ':!fixtures' \
  ':!connectors.lock' ':!crates/catalog-reader/catalog.pack' \
  ':!crates/integration-b10x' ':!docs/stories/S-*.md' \
  ':!scripts/as-bot.sh' ':!scripts/bot-token.sh' ':!scripts/bot-gh.sh' \
  ':!scripts/check-bot-files.py' ':!scripts/vendor-*' \
  ':!scripts/check-brand.sh' \
  | grep -viE "$allow" || true)
if test -n "$hits"; then
  printf 'brand check: b10x at the surface:\n%s\n' "$hits" >&2
  exit 1
fi
printf 'brand check: clean\n'
