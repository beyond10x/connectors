#!/bin/bash
# The shipped GitLab selection set on the catalog provider built after the native
# GitLab adapter was deleted: the same eleven reads, then three guarded write
# attempts that must all be refused before dispatch, because merge request 10 is
# already merged and merge request 999 does not exist. Fresh private
# configuration cfg9/state9; the run of 2026-09-13 (shipped.sh) is the record of
# the applied merge with the same engine code.
set -u
SB=~/.cache/connectors-gitlab-20260912
REPO=${REPO:-$HOME/beyond10x/connectors_v2}
CLI="$REPO/target/release/connectors --output json --config $SB/cfg9/config.toml --state-dir $SB/state9"
LOG="$SB/journey/retire.log"; : > "$LOG"
J="$SB/journey"; P="$SB/private"; PROJ=root/connectors-sandbox
umask 077
say() { echo "$*" | tee -a "$LOG"; }
jget() { python3 -c "import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1]))" "$1"; }
head_of() { curl -sS -K "$SB/curlrc" "https://localhost:8929/api/v4/projects/1/repository/branches/$(printf %s "$1" | sed 's#/#%2F#g')" | jget "d['commit']['id']"; }
merge_puts() { docker exec connectors-gitlab-20260912 sh -c "grep -c \"PUT /api/v4/projects/root%2Fconnectors-sandbox/merge_requests/$1/merge \" /var/log/gitlab/nginx/gitlab_access.log"; }
puts() { docker exec connectors-gitlab-20260912 sh -c "grep -c \"PUT /api/v4/projects/root%2Fconnectors-sandbox/merge_requests/$1 \" /var/log/gitlab/nginx/gitlab_access.log"; }
summary() { python3 -c "import json,sys
d=json.load(open(sys.argv[1]))
r=d.get('result') if d.get('ok') else d.get('error',{}).get('data',{})
m=r.get('mutation',{}) if isinstance(r,dict) else {}
body=''
if d.get('ok') and isinstance(r,dict) and 'result' in r:
    try:
        v=json.loads(r['result']); b=v.get('body'); body=' status '+str(v.get('status'))+' body '+(('text:'+repr(b[:40])) if isinstance(b,str) else ('list['+str(len(b))+']' if isinstance(b,list) else 'object'))
    except Exception as e: body=' (unparsed result)'
print(('ok'+body) if d.get('ok') else 'ERR '+str(d.get('error',{}).get('code'))+' '+str((d.get('error',{}).get('data') or {}).get('code'))+' stage '+str((d.get('error',{}).get('data') or {}).get('stage')), 'classification', m.get('classification'), 'cause', m.get('cause'))" "$1"; }

# 1. private configuration: catalog provider over the shipped selection set
rm -rf "$SB/cfg9" "$SB/state9"; mkdir -p "$SB/cfg9" "$SB/state9"; chmod 700 "$SB/cfg9" "$SB/state9"
# A proof is issued to a new path only; clear the previous run's proofs.
rm -f "$P"/proof-ret-*.json
$CLI setup init > "$J/retire-setup-init.json" 2>&1; say "setup init exit=$?"
EXE="$REPO/target/release/connectors-catalog-provider"
NEWSHA=$(sha256sum "$EXE" | cut -d' ' -f1)
REV=$("$EXE" --local-config "$SB/gitlab-catalog-2.json" --print-local-bootstrap | jget "d['configuration_revision']")
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
operations = ["project.get", "issues.list", "file.get", "branch.get", "merge_requests.list", "merge_request.get", "pipelines.list", "pipeline.get", "pipeline.jobs", "job.get", "job.trace", "merge_request.create", "merge_request.update", "merge_request.merge"]

[adapters.forge.executable]
path = "$EXE"
sha256 = "$NEWSHA"
args = ["--local-config", "$SB/gitlab-catalog-2.json"]
TOML
} > "$SB/cfg9/config.toml"
say "executable sha256 $NEWSHA configuration_revision $REV"
$CLI setup check > "$J/retire-setup.json" 2>&1; say "setup check exit=$?"
CONN=$($CLI connections connect --adapter forge --profile gitlab.pat --credential-file "$SB/credential-dev-api.json" 2>&1 | tee "$SB/conn9.json" | jget "d['result']['connection']['summary']['connection']")
say "connection $CONN identity $(jget "d['result']['connection']['external_identity']" < "$SB/conn9.json")"
$CLI approvals key-init --adapter forge > "$J/retire-keyinit.json" 2>&1; say "key-init exit=$?"
printf '%s' '{"operations":["merge_request.create","merge_request.update","merge_request.merge"]}' > "$P/policy-retire.json"
$CLI approvals policy-set --adapter forge --input-file "$P/policy-retire.json" > "$J/retire-policy.json" 2>&1; say "policy-set exit=$?"
$CLI operations describe --adapter forge --operation merge_request.merge > "$J/retire-describe-merge.json" 2>&1
say "merge_request.merge input required: $(jget "json.loads(d['result']['schema'])['required'] if isinstance(d['result']['schema'],str) else d['result']['schema'].get('required')" < "$J/retire-describe-merge.json" 2>/dev/null)"

describe() { $CLI operations describe --adapter forge --operation "$1" | jget "d['result']['schema']+' '+d['result']['revision']"; }
read_op() { # name op input-json
  local s r; read s r < <(describe "$2")
  $CLI operations invoke --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" --input-json "$3" > "$J/retire-$1.json" 2>&1
  say "$1 ($2): $(summary "$J/retire-$1.json")"
}
write_op() { # name op input-file key
  local s r d; read s r < <(describe "$2")
  d=$($CLI approvals prepare --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" --input-file "$3" | jget "d['result']['preparation']['subject_sha256']")
  $CLI approvals issue --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" --input-file "$3" --approve-subject "$d" --proof-output "$P/proof-ret-$1.json" >/dev/null
  $CLI operations invoke --adapter forge --connection "$CONN" --operation "$2" --schema "$s" --revision "$r" \
    --input-file "$3" --approval-file "$P/proof-ret-$1.json" --idempotency-key "$4" > "$J/retire-$1.json" 2>&1
  say "$1 ($2): $(summary "$J/retire-$1.json")"
}

# 2. every read the native adapter exposes
read_op project project.get "{\"id\":\"$PROJ\"}"
read_op issues issues.list "{\"id\":\"$PROJ\",\"per_page\":2}"
read_op file file.get "{\"id\":\"$PROJ\",\"file_path\":\".gitlab-ci.yml\",\"ref\":\"main\"}"
read_op branch branch.get "{\"id\":\"$PROJ\",\"branch\":\"main\"}"
read_op mrs merge_requests.list "{\"id\":\"$PROJ\",\"state\":\"opened\",\"per_page\":3}"
read_op mr-10 merge_request.get "{\"id\":\"$PROJ\",\"merge_request_iid\":10}"
say "MR 10 head $(jget "json.loads(d['result']['result'])['body']['sha']" < "$J/retire-mr-10.json") pipeline $(jget "(json.loads(d['result']['result'])['body'].get('head_pipeline') or {}).get('id')" < "$J/retire-mr-10.json") $(jget "(json.loads(d['result']['result'])['body'].get('head_pipeline') or {}).get('status')" < "$J/retire-mr-10.json") $(jget "json.loads(d['result']['result'])['body']['detailed_merge_status']" < "$J/retire-mr-10.json")"
read_op pipelines pipelines.list "{\"id\":\"$PROJ\",\"per_page\":2}"
read_op pipeline-19 pipeline.get "{\"id\":\"$PROJ\",\"pipeline_id\":19}"
read_op pipeline-19-jobs pipeline.jobs "{\"id\":\"$PROJ\",\"pipeline_id\":19}"
JOB=$(jget "json.loads(d['result']['result'])['body'][0]['id']" < "$J/retire-pipeline-19-jobs.json"); say "job of pipeline 19: $JOB"
read_op job job.get "{\"id\":\"$PROJ\",\"job_id\":$JOB}"
read_op job-trace job.trace "{\"id\":\"$PROJ\",\"job_id\":$JOB}"

# 3. writes on the post-removal binary: every attempt refused before dispatch
# The source branch of MR 10 was removed at its merge; the pinned head is the sha GitLab records on the merged request.
A=$(jget "json.loads(d['result']['result'])['body']['sha']" < "$J/retire-mr-10.json"); M=$(head_of main); say "MR 10 recorded head $A; main head $M"
MP0=$(merge_puts 10); U0=$(puts 10); say "PUTs to 10 before: merge $MP0 update $U0"
printf '%s' "{\"id\":\"$PROJ\",\"merge_request_iid\":10,\"pipeline_id\":19,\"body\":{\"sha\":\"$A\"}}" > "$P/ret-merge-merged.json"
write_op merge-merged merge_request.merge "$P/ret-merge-merged.json" ret-merge-merged-20260914
say "merge PUTs to 10 now: $(merge_puts 10) (before $MP0)"
printf '%s' "{\"id\":\"$PROJ\",\"merge_request_iid\":999,\"pipeline_id\":19,\"body\":{\"sha\":\"$A\"}}" > "$P/ret-merge-missing.json"
write_op merge-missing merge_request.merge "$P/ret-merge-missing.json" ret-merge-missing-20260914
say "merge PUTs to 999: $(merge_puts 999)"
printf '%s' "{\"id\":\"$PROJ\",\"merge_request_iid\":10,\"sha\":\"$A\",\"body\":{\"title\":\"Retitled after retirement\"}}" > "$P/ret-update-merged.json"
write_op update-merged merge_request.update "$P/ret-update-merged.json" ret-update-merged-20260914
say "PUTs to 10 now: merge $(merge_puts 10) update $(puts 10) (before merge $MP0 update $U0)"
curl -sS -K "$SB/curlrc" "https://localhost:8929/api/v4/projects/1/merge_requests/10" | python3 -c "import json,sys; m=json.load(sys.stdin); print('MR', m['iid'], m['state'], m['sha'], 'merge_commit', m.get('merge_commit_sha'), repr(m['title']))" | tee -a "$LOG"
echo DONE >> "$LOG"
