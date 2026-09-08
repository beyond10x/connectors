# Final checkpoint audit transcript

One-off evidence audit, not executable project tooling or runtime conformance. Exit 0.

```python
from pathlib import Path
import json,tarfile,hashlib,re,subprocess,collections,yaml
base=Path('docs/evidence/mutation-profiles-20260908');archives=0;projection_entries=0;current=[]
for who in ['a','b']:
 for phase in ['initial','classification-recheck1','classification-recheck2']:
  arcpath=base/'reviews'/f'{who}-{phase}-packet.tar.gz'
  with tarfile.open(arcpath) as arc:
   names=set(arc.getnames());manifests=['source-hashes.json']
   if who=='a' and phase=='initial':manifests+=['supplemental-source-hashes.json']
   if who=='b' and phase=='initial':manifests+=['historical-hashes.json']
   for name in manifests:
    raw=json.load(arc.extractfile(name));rows=raw['files'] if isinstance(raw,dict) else raw
    for row in rows:
     snap=row.get('snapshot') or ('historical/' if name.startswith('historical') else 'snapshot/')+row['path']
     data=arc.extractfile(snap).read();assert hashlib.sha256(data).hexdigest()==row['sha256'],(arcpath,snap);assert len(data)==row['bytes'];archives+=1
     if phase=='classification-recheck2':
      source=row.get('source',row.get('path'));p=Path(source)
      if not source.startswith('../connectors@'):
       assert p.is_file(),source;assert hashlib.sha256(p.read_bytes()).hexdigest()==row['sha256'],source;current.append({'reviewer':who,'source':source,'sha256':row['sha256']})
   if 'projection-source-hashes.json' in names:
    for row in json.load(arc.extractfile('projection-source-hashes.json'))['files']:
     data=arc.extractfile(row['snapshot']).read();assert hashlib.sha256(data).hexdigest()==row['sha256'];assert len(data)==row['bytes'];projection_entries+=1
  slug=f'mutation-profiles-{who}-initial-20260908' if phase=='initial' else f'mutation-classification-{who}-{phase.removeprefix("classification-")}-20260908'
  p=Path('.engineering/planning/review-result')/(slug+'.md');body=p.read_text().split('---',2)[2].strip();raw=(base/'reviews'/f'{who}-{phase}-report.md').read_text().strip();assert body==raw,slug
outcomes=json.loads((base/'classification-finding-outcomes.json').read_text());journal=[json.loads(l) for l in Path('.engineering/planning/journal.jsonl').read_text().splitlines()]
for row in outcomes:
 found=[r for r in journal if r.get('id')=='contracts-mutation-classification' and r.get('args',{}).get('kind')=='review_outcome' and r['args'].get('reference')==row['ref']]
 assert len(found)==1,row['id'];assert found[0]['args']['review']==row['review'];assert found[0]['args']['outcome']==row['outcome']
ledger=Path('.engineering/planning/specification/contract-review-intake-20260908.md').read_text();rows=[l for l in ledger.splitlines() if re.match(r'\| [FE]\d\d \|',l)];ids=[l.split('|')[1].strip() for l in rows]
assert len(rows)==48 and len(set(ids))==48
fixed=sum(bool(re.search(r'\| Fixed\b',l)) for l in rows);assert fixed==29,fixed
statuses=collections.Counter()
for p in Path('.engineering/planning/story').glob('contracts-*.md'):
 v=yaml.safe_load(p.read_text().split('---',2)[1]);statuses[v['status']]+=1
assert dict(statuses)=={'implemented':18,'draft':9},statuses
changed=subprocess.check_output(['git','diff','--name-only','c7cd05a'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
assert not any(p.startswith(('crates/','adapters/','spec-kinds/')) or (p.endswith('.rs') and not p.startswith('docs/evidence/')) for p in changed)
links=0
for rel in ['contracts/operations/v1alpha1/semantics.md','contracts/sessions/v1alpha1/semantics.md','docs/adapters/media-session.md','docs/adapters/atlassian.md',str(base/'classification-verification.md'),str(base/'classification-dispositions.md')]:
 p=Path(rel)
 for target in re.findall(r'(?<!!)\[[^\]]*\]\(([^)]+)\)',p.read_text()):
  target=target.strip('<>').split('#')[0]
  if not target or re.match(r'[a-zA-Z]+:',target):continue
  assert (p.parent/target).exists(),(rel,target);links+=1
result={'archived_source_entries_verified':archives,'archived_projection_entries_verified':projection_entries,'current_final_review_hashes_verified':len(current),'raw_review_bodies_equal':6,'individual_outcomes_exactly_once':len(outcomes),'original_ledger_findings':48,'fixed':fixed,'open':48-fixed,'story_statuses':dict(statuses),'local_links_checked':links,'runtime_or_adapter_schema_changes':False,'current_final_sources':current}
(base/'classification-final-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='current_final_sources'},indent=2))
```

```json
{
  "archived_source_entries_verified": 305,
  "archived_projection_entries_verified": 430,
  "current_final_review_hashes_verified": 88,
  "raw_review_bodies_equal": 6,
  "individual_outcomes_exactly_once": 5,
  "original_ledger_findings": 48,
  "fixed": 29,
  "open": 19,
  "story_statuses": {
    "draft": 9,
    "implemented": 18
  },
  "local_links_checked": 53,
  "runtime_or_adapter_schema_changes": false
}
```
