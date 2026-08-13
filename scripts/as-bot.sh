#!/usr/bin/env bash
# Run any git command as selfdirect-bot. Examples:
#   scripts/as-bot.sh commit -m "chore: bump catalog lock"
#   scripts/as-bot.sh push origin main
#
# Commits are authored as selfdirect-bot[bot]; pushes authenticate with a fresh
# 1-hour installation token (scripts/bot-token.sh) over https, regardless of the
# remote's configured protocol (ssh remotes are rewritten for the push). The
# token travels via environment + credential helper, never via argv.
#
# This shows the bot identity on commits but not GitHub's "Verified" badge —
# only API-created commits get that. For routine automation this is the way.
set -euo pipefail

dir="$(cd "$(dirname "$0")/.." && pwd)"
SD_BOT_TOKEN="$("$dir/scripts/bot-token.sh")"
export SD_BOT_TOKEN

exec git \
  -c user.name='selfdirect-bot[bot]' \
  -c user.email='4575767+selfdirect-bot[bot]@users.noreply.github.com' \
  -c 'url.https://github.com/.pushInsteadOf=git@github.com:' \
  -c 'credential.https://github.com.helper=' \
  -c 'credential.https://github.com.helper=!f() { echo username=x-access-token; echo "password=${SD_BOT_TOKEN}"; }; f' \
  "$@"
