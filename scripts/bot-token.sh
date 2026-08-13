#!/usr/bin/env bash
# Mint a short-lived (1h) installation token for the selfdirect-bot GitHub App.
#
# The token is printed to stdout, alone; diagnostics go to stderr. Use it as
#   git push https://x-access-token:$(scripts/bot-token.sh)@github.com/selfdirect/<repo>.git
# or as an Authorization: token header. Commits authored as
#   selfdirect-bot[bot] <APP_ID+selfdirect-bot[bot]@users.noreply.github.com>
# display with the bot identity; commits created via the GraphQL
# createCommitOnBranch API are additionally signature-verified by GitHub.
#
# Works with zero configuration. Env overrides (optional):
#   SELFDIRECT_BOT_APP_ID  GitHub App id        (default: 4575767)
#   SELFDIRECT_BOT_KEY     private key PEM path (default: newest
#                          ~/selfdirect/selfdirect-bot.*.private-key.pem;
#                          keep it 0600, outside every git working tree)
#   SELFDIRECT_BOT_ORG     installation account (default: selfdirect)
set -euo pipefail

app_id="${SELFDIRECT_BOT_APP_ID:-4575767}"
key="${SELFDIRECT_BOT_KEY:-}"
org="${SELFDIRECT_BOT_ORG:-selfdirect}"
if [ -z "$key" ]; then
  key="$(ls -t "$HOME"/selfdirect/selfdirect-bot.*.private-key.pem 2>/dev/null | head -1)"
fi
[ -n "$key" ] && [ -r "$key" ] || { echo "no readable bot key (looked for ~/selfdirect/selfdirect-bot.*.private-key.pem; override with SELFDIRECT_BOT_KEY)" >&2; exit 1; }

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }

now="$(date +%s)"
header="$(printf '{"alg":"RS256","typ":"JWT"}' | b64url)"
payload="$(printf '{"iat":%d,"exp":%d,"iss":"%s"}' "$((now - 60))" "$((now + 540))" "$app_id" | b64url)"
sig="$(printf '%s.%s' "$header" "$payload" | openssl dgst -sha256 -sign "$key" -binary | b64url)"
jwt="$header.$payload.$sig"

api() { curl -sfS -H "Authorization: Bearer $jwt" -H "Accept: application/vnd.github+json" "$@"; }

installation_id="$(api https://api.github.com/app/installations \
  | jq -r --arg org "$org" '.[] | select(.account.login == $org) | .id')"
[ -n "$installation_id" ] || { echo "app is not installed on '$org'" >&2; exit 1; }

api -X POST "https://api.github.com/app/installations/${installation_id}/access_tokens" \
  | jq -r .token
