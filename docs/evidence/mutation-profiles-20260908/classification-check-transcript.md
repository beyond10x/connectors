# One-off classification evidence check

This is an audit transcript, not installed project tooling or a runtime test. Python/jsonschema checks generic generated value shapes and exact archived bytes only.

```python
from pathlib import Path
import hashlib,json,shutil,tarfile
from jsonschema import Draft202012Validator
base=Path('docs/evidence/mutation-profiles-20260908')
a=Path('.local/mutation-profiles-20260908/classification-schema-a');b=Path('.local/mutation-profiles-20260908/classification-schema-b')
files=sorted(p.relative_to(a) for p in a.rglob('*') if p.is_file())
assert files==sorted(p.relative_to(b) for p in b.rglob('*') if p.is_file())
projections=[]
for rel in files:
 data=(a/rel).read_bytes();assert data==(b/rel).read_bytes()
 projections.append({'path':str(rel),'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
(base/'classification-projection-hashes.json').write_text(json.dumps(projections,indent=2)+'\n')
types=['ExecutableEffect','SemanticEffect','EffectDeclaration','SessionEstablishmentObservation'];dest=base/'classification-schema'/'types';dest.mkdir(parents=True,exist_ok=True)
for name in types:
 f='connectors.mutations.'+name+'.schema.json';shutil.copyfile(a/'schema'/'types'/f,dest/f)
cases=[]
def case(id,type,value,accept=True,semantic=None,note='Shape only; no semantic predicate executed.'):
 cases.append(dict(id=id,type='connectors.mutations.'+type,value=value,schema_accepts=accept,semantic_accepts=semantic,note=note))
for x in ['external_write','network','process','local_system','send_external','session_establishment']:case('effect-'+x,'ExecutableEffect',x)
for x in ['human_visible','billable','irreversible']:case('hint-'+x,'SemanticEffect',x)
sip={'profile':'mutation','effects':['external_write','network','send_external','session_establishment'],'semantic_effects':['human_visible']}
case('sip-valid','EffectDeclaration',sip,semantic=True)
case('read-network','EffectDeclaration',{'profile':'kubernetes-list','effects':['network'],'semantic_effects':[]},semantic=True)
case('missing-hints','EffectDeclaration',{'profile':'mutation','effects':['external_write']},False)
case('unknown-executable','EffectDeclaration',{**sip,'effects':['external_write','write']},False)
case('hint-in-executable','EffectDeclaration',{**sip,'effects':['external_write','human_visible']},False)
case('executable-in-hint','EffectDeclaration',{**sip,'semantic_effects':['send_external']},False)
case('missing-discriminator','EffectDeclaration',{**sip,'effects':['network','send_external','session_establishment']},True,False,'Counterexample: shape-valid but mutation discriminator is missing; future semantic validator must refuse.')
case('read-business-write','EffectDeclaration',{**sip,'profile':'kubernetes-list'},True,False,'Counterexample: a read cannot claim external business mutation; schema does not execute profile compatibility.')
case('duplicate-effect','EffectDeclaration',{**sip,'effects':['external_write','external_write']},True,False,'Counterexample: semantic distinct-member rule is not projected by List<T>.')
for cls in ['not_attempted','refused','applied','unknown']:case('no-receipt-'+cls,'SessionEstablishmentObservation',{'mutation':{'classification':cls}},semantic=True)
case('applied-ready','SessionEstablishmentObservation',{'mutation':{'classification':'applied'},'ready_session':'sess-a'},semantic=True)
case('unknown-ready','SessionEstablishmentObservation',{'mutation':{'classification':'unknown'},'ready_session':'sess-a'},True,False,'Counterexample: a trusted ready receipt requires applied knowledge; the generic schema does not check the relation.')
case('null-ready','SessionEstablishmentObservation',{'mutation':{'classification':'applied'},'ready_session':None},False,note='Private Optional<T> projects omission, not public required-null encoding.')
case('replayed-classification','SessionEstablishmentObservation',{'mutation':{'classification':'replayed'}},False)
case('missing-effect-observation','SessionEstablishmentObservation',{},False)
results=[]
for row in cases:
 schema=json.loads((dest/(row['type']+'.schema.json')).read_text());Draft202012Validator.check_schema(schema)
 errors=list(Draft202012Validator(schema).iter_errors(row['value']));got=not errors
 assert got==row['schema_accepts'],row['id']
 results.append({'id':row['id'],'schema_accepted':got,'errors':[e.message for e in errors]})
(base/'classification-values.json').write_text(json.dumps(cases,indent=2)+'\n');(base/'classification-shape-results.json').write_text(json.dumps(results,indent=2)+'\n')
# Verify every archived reviewer source against its own manifest, not current files.
checks=[]
for who in ['a','b']:
 with tarfile.open(base/'reviews'/f'{who}-initial-packet.tar.gz') as arc:
  if who=='a':
   for manifest in ['source-hashes.json','supplemental-source-hashes.json']:
    v=json.load(arc.extractfile(manifest))
    for row in v['files']:
     data=arc.extractfile(row['snapshot']).read(); assert hashlib.sha256(data).hexdigest()==row['sha256'];assert len(data)==row['bytes'];checks.append({'reviewer':who,'snapshot':row['snapshot'],'sha256':row['sha256']})
  else:
   for manifest,prefix in [('source-hashes.json','snapshot/'),('historical-hashes.json','historical/')]:
    v=json.load(arc.extractfile(manifest));rows=v['files'] if isinstance(v,dict) else v
    for row in rows:
     data=arc.extractfile(prefix+row['path']).read();assert hashlib.sha256(data).hexdigest()==row['sha256'];assert len(data)==row['bytes'];checks.append({'reviewer':who,'snapshot':prefix+row['path'],'sha256':row['sha256']})
(base/'initial-source-audit.json').write_text(json.dumps(checks,indent=2)+'\n')
print(json.dumps({'schema_artifacts_compared':len(files),'values_checked':len(cases),'shape_negative':sum(not x['schema_accepts'] for x in cases),'accepted_semantic_counterexamples':sum(x['schema_accepts'] and x['semantic_accepts'] is False for x in cases),'archived_initial_sources_verified':len(checks)},indent=2))
```

Observed exit: 0. Output:

```json
{
  "schema_artifacts_compared": 215,
  "values_checked": 27,
  "shape_negative": 7,
  "accepted_semantic_counterexamples": 4,
  "archived_initial_sources_verified": 125
}
```
