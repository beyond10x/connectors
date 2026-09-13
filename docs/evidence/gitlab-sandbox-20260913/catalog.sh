#!/bin/bash
# GitLab merge-request reads and writes through the catalog provider: the pinned
# OpenAPI source, a declarative selection, no per-endpoint Rust. Fresh private
# configuration cfg7/state7.
set -u
SB=~/.cache/connectors-gitlab-20260912
REPO=${REPO:-$HOME/beyond10x/connectors_v2}
CLI="$REPO/target/release/connectors --output json --config $SB/cfg7/config.toml --state-dir $SB/state7"
LOG="$SB/journey/catalog.log"; : > "$LOG"
J="$SB/journey"; P="$SB/private"; PROJ=root/connectors-sandbox
umask 077
say() { echo "$*" | tee -a "$LOG"; }
jget() { python3 -c "import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1]))" "$1"; }
head_of() { curl -sS -K "$SB/curlrc" "https://localhost:8929/api/v4/projects/1/repository/branches/$(printf %s "$1" | sed 's#/#%2F#g')" | jget "d['commit']['id']"; }
posts() { docker exec connectors-gitlab-20260912 sh -c 'grep -c "POST /api/v4/projects/root%2Fconnectors-sandbox/merge_requests " /var/log/gitlab/nginx/gitlab_access.log'; }
puts() { docker exec connectors-gitlab-20260912 sh -c "grep -c \"PUT /api/v4/projects/root%2Fconnectors-sandbox/merge_requests/$1 \" /var/log/gitlab/nginx/gitlab_access.log"; }
summary() { python3 -c "import json,sys
d=json.load(open(sys.argv[1]))
r=d.get('result') if d.get('ok') else d.get('error',{}).get('data',{})
m=r.get('mutation',{}) if isinstance(r,dict) else {}
print('ok' if d.get('ok') else 'ERR '+str(d.get('error',{}).get('code'))+' '+str((d.get('error',{}).get('data') or {}).get('code'))+' stage '+str((d.get('error',{}).get('data') or {}).get('stage')), 'classification', m.get('classification'), 'cause', m.get('cause'))" "$1"; }

# 1. private configuration: catalog provider bound to the GitLab bundle
rm -rf "$SB/cfg7" "$SB/state7"; mkdir -p "$SB/cfg7" "$SB/state7"; chmod 700 "$SB/cfg7" "$SB/state7"
# setup init creates the metadata store and a default configuration; the
# configuration is then replaced with the composed one and re-checked.
$CLI setup init > "$J/catalog-setup-init.json" 2>&1; say "setup init exit=$?"
EXE="$REPO/target/release/connectors-catalog-provider"
NEWSHA=$(sha256sum "$EXE" | cut -d' ' -f1)
REV=$("$EXE" --local-config "$SB/gitlab-catalog.json" --print-local-bootstrap | jget "d['configuration_revision']")
{
  sed -n '1,/^\[/p' "$SB/cfg5/config.toml" | grep -E '^(format|owner_uid|secret_service_socket) ='
  echo
  echo '[adapters]'
  echo
  echo '[approval_clock]'
  sed -n '/^\[approval_clock\]/,/^$/p' "$SB/cfg5/config.toml" | grep -E '^(format|address|public_key|max_rate_error_ppm) ='
  cat <<TOML

[adapters.forge]
instance_id = "gitlab-sandbox"
adapter_id = "catalog"
configuration_revision = "$REV"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[adapters.forge.permissions]
profiles = ["gitlab.pat"]
operations = ["merge_request.get", "branch.get", "merge_request.create", "merge_request.update"]

[adapters.forge.executable]
path = "$EXE"
sha256 = "$NEWSHA"
args = ["--local-config", "$SB/gitlab-catalog.json"]
TOML
} > "$SB/cfg7/config.toml"
say "executable sha256 $NEWSHA configuration_revision $REV"
$CLI setup check > "$J/catalog-setup.json" 2>&1; say "setup check exit=$?"
CONN=$($CLI connections connect --adapter forge --profile gitlab.pat --credential-file "$SB/credential-dev-api.json" 2>&1 | tee "$SB/conn7.json" | jget "d['result']['connection']['summary']['connection']")
say "connection $CONN identity $(jget "d['result']['connection']['external_identity']" < "$SB/conn7.json")"
$CLI approvals key-init --adapter forge > "$J/catalog-keyinit.json" 2>&1; say "key-init exit=$?"
printf '%s' '{"operations":["merge_request.create","merge_request.update"]}' > "$P/policy-catalog.json"
$CLI approvals policy-set --adapter forge --input-file "$P/policy-catalog.json" > "$J/catalog-policy.json" 2>&1; say "policy-set exit=$?"

describe() { $CLI operations describe --adapter forge --operation "$1" | jget "d['result']['schema']+' '+d['result']['revision']"; }
read_op() { # name op input-json
  local s r; read s r < <(describe "$2")
  $CLI operations invoke --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" --input-json "$3" > "$J/catalog-$1.json" 2>&1
  say "$1 ($2): $(summary "$J/catalog-$1.json")"
}
write_op() { # name op input-file key
  local s r d; read s r < <(describe "$2")
  d=$($CLI approvals prepare --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" --input-file "$3" | jget "d['result']['preparation']['subject_sha256']")
  $CLI approvals issue --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" --input-file "$3" --approve-subject "$d" --proof-output "$P/proof-cat-$1.json" >/dev/null
  $CLI operations invoke --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" \
    --input-file "$3" --approval-file "$P/proof-cat-$1.json" --idempotency-key "$4" > "$J/catalog-$1.json" 2>&1
  say "$1 ($2): $(summary "$J/catalog-$1.json")"
}

# 2. reads through the bundle
read_op get-9 merge_request.get "{\"id\":\"$PROJ\",\"merge_request_iid\":9}"
read_op branch-applied branch.get "{\"id\":\"$PROJ\",\"branch\":\"feature/create-applied\"}"
read_op get-outside merge_request.get "{\"id\":\"$PROJ\",\"merge_request_iid\":999999}"

POSTS0=$(posts); say "POSTs before: $POSTS0"
# 3. create: applied
A=$(head_of feature/create-applied); say "pinned head (applied) $A"
printf '%s' "{\"id\":\"$PROJ\",\"sha\":\"$A\",\"body\":{\"source_branch\":\"feature/create-applied\",\"target_branch\":\"main\",\"title\":\"Opened through the catalog provider\"}}" > "$P/cat-create-applied.json"
write_op create-applied merge_request.create "$P/cat-create-applied.json" cat-create-applied-20260913
IID=$(jget "json.loads(d['result']['result'])['body']['iid']" < "$J/catalog-create-applied.json" 2>/dev/null); say "opened merge request iid ${IID:-?} POSTs now $(posts)"
# 4. create: stale pin (main's head is a real commit, not the branch head)
M=$(head_of main); say "stale pin $M"
printf '%s' "{\"id\":\"$PROJ\",\"sha\":\"$M\",\"body\":{\"source_branch\":\"feature/create-applied\",\"target_branch\":\"main\",\"title\":\"Opened at a stale pin\"}}" > "$P/cat-create-stale.json"
write_op create-stale merge_request.create "$P/cat-create-stale.json" cat-create-stale-20260913
say "POSTs now $(posts)"
# 5. create: same source branch again, GitLab answers 409
printf '%s' "{\"id\":\"$PROJ\",\"sha\":\"$A\",\"body\":{\"source_branch\":\"feature/create-applied\",\"target_branch\":\"main\",\"title\":\"Opened twice\"}}" > "$P/cat-create-conflict.json"
write_op create-conflict merge_request.create "$P/cat-create-conflict.json" cat-create-conflict-20260913
say "POSTs now $(posts)"
# 6. update the opened request: applied, then stale
if [ -n "${IID:-}" ]; then
  printf '%s' "{\"id\":\"$PROJ\",\"merge_request_iid\":$IID,\"sha\":\"$A\",\"body\":{\"title\":\"Retitled through the catalog provider\"}}" > "$P/cat-update-applied.json"
  write_op update-applied merge_request.update "$P/cat-update-applied.json" cat-update-applied-20260913
  say "PUTs to $IID: $(puts $IID)"
  printf '%s' "{\"id\":\"$PROJ\",\"merge_request_iid\":$IID,\"sha\":\"$M\",\"body\":{\"title\":\"Retitled at a stale pin\"}}" > "$P/cat-update-stale.json"
  write_op update-stale merge_request.update "$P/cat-update-stale.json" cat-update-stale-20260913
  say "PUTs to $IID: $(puts $IID)"
fi
# 7. create: raced — move the branch the moment the preflight GET appears
C=$(head_of feature/create-raced); say "pinned head (raced) $C"
printf '%s' "{\"id\":\"$PROJ\",\"sha\":\"$C\",\"body\":{\"source_branch\":\"feature/create-raced\",\"target_branch\":\"main\",\"title\":\"Opened while the head was moving\"}}" > "$P/cat-create-raced.json"
$CLI adapters status --adapter forge >/dev/null 2>&1
(
  docker exec connectors-gitlab-20260912 sh -c 'tail -n 0 -F /var/log/gitlab/nginx/gitlab_access.log' 2>/dev/null |
  while IFS= read -r line; do
    case "$line" in
      *"GET /api/v4/projects/root%2Fconnectors-sandbox/repository/branches/feature%2Fcreate-raced"*)
        python3 - <<'PY' >> "$LOG" 2>&1 &
import json, ssl, os, pathlib, urllib.request
SB = pathlib.Path(os.path.expanduser("~/.cache/connectors-gitlab-20260912"))
TOK = json.loads((SB/"credential.json").read_text())["token"]
CTX = ssl.create_default_context(cafile=str(SB/"ssl"/"ca.crt"))
body = json.dumps({"branch":"feature/create-raced","commit_message":"Move the head while a create is in flight",
  "actions":[{"action":"create","file_path":"raced-create.txt","content":"raced\n"}]}).encode()
r = urllib.request.Request("https://localhost:8929/api/v4/projects/1/repository/commits", data=body, method="POST")
r.add_header("PRIVATE-TOKEN", TOK); r.add_header("Content-Type","application/json")
with urllib.request.urlopen(r, context=CTX) as x:
    print("moved head to", json.loads(x.read())["id"])
PY
        break;;
    esac
  done
) &
W=$!
sleep 1
write_op create-raced merge_request.create "$P/cat-create-raced.json" cat-create-raced-20260913
sleep 3; kill $W 2>/dev/null
say "branch head after race $(head_of feature/create-raced)"
say "POSTs total: $(posts) (before $POSTS0)"
curl -sS -K "$SB/curlrc" "https://localhost:8929/api/v4/projects/1/merge_requests?state=all&per_page=50" | python3 -c "import json,sys; [print('MR', m['iid'], m['state'], m['source_branch'], m['sha'], repr(m['title'])) for m in json.load(sys.stdin) if m['iid']>9]" | tee -a "$LOG"
echo DONE >> "$LOG"
