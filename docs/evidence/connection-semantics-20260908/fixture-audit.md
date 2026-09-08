# One-off type-projection inspection

This transcript constructs declared scenario values and checks only their generated ESS schema shape. It executes no semantic reduction, current authorization, provider, store or management flow. The contradictory eligible/error example deliberately demonstrates that valid shape is not valid admission. This is verification evidence, not installed executable project tooling.

```python
from pathlib import Path
import json,copy,hashlib,jsonschema
out=Path('docs/evidence/connection-semantics-20260908');root=Path('.local/connection-semantics-20260908')
# Declared semantic examples. This script checks type shape, not the reduction.
base=dict(metadata_available=True,locally_revoked=False,enabled=True,established_credential_failure=False,required_custody_available=True,required_parent_available=True,validated_publication=True,baseline_current=True)
viability=[]
for ident,changes,state in [
 ('baseline-ready',{},'ready'),('new-unpublished',{'validated_publication':False},'pending'),('stale-baseline',{'baseline_current':False},'pending'),('unknown-identity',{'baseline_current':False},'pending'),('provider-credential-revoked',{'established_credential_failure':True},'reauthorization_required'),('consumed-refresh',{'established_credential_failure':True,'baseline_current':False},'reauthorization_required'),('custody-unavailable',{'required_custody_available':False},'custody_unavailable'),('parent-degraded',{'required_parent_available':False},'parent_degraded'),('disabled',{'enabled':False},'disabled'),('terminal-revoked',{'locally_revoked':True},'revoked'),('revoked-all-failures',{'locally_revoked':True,'enabled':False,'established_credential_failure':True,'required_custody_available':False,'required_parent_available':False},'revoked'),('disabled-all-failures',{'enabled':False,'established_credential_failure':True,'required_custody_available':False},'disabled'),('known-invalid-with-custody-outage',{'established_credential_failure':True,'required_custody_available':False},'reauthorization_required'),('custody-before-parent',{'required_custody_available':False,'required_parent_available':False},'custody_unavailable'),('parent-before-baseline',{'required_parent_available':False,'baseline_current':False},'parent_degraded'),('metadata-unavailable',{'metadata_available':False},None)]:
 facts=base|changes;decision={'state':state} if state else {'error':'unavailable'}
 viability.append({'id':ident,'facts':facts,'expected':decision})
eligbase=dict(authority_available=True,host_admitted=True,operation_enabled=True,viability={'state':'ready'},profile_matches=True,scope='satisfied',permission='not_required',verification='not_required')
eligibility=[]
for ident,changes,error in [
 ('read-without-optional-write',{},None),('write-missing-grant',{'scope':'insufficient'},'insufficient_scope'),('unknown-operation-grants',{'scope':'unknown'},'connection_not_ready'),('target-permission-denied',{'permission':'denied'},'forbidden'),('target-permission-stale',{'permission':'stale'},'unavailable'),('permission-unavailable',{'permission':'unavailable'},'unavailable'),('operation-verification-not-current',{'verification':'stale'},'connection_not_ready'),('required-verification-unknown',{'verification':'unknown'},'connection_not_ready'),('optional-verification-not-required',{},None),('current-grant-denied',{'host_admitted':False},'not_granted'),('grant-policy-unavailable',{'authority_available':False},'unavailable'),('operation-disabled',{'operation_enabled':False},'forbidden'),('profile-purpose-mismatch',{'profile_matches':False},'forbidden'),('explicit-configured-no-alternatives',{'scope':'not_required'},None),('pending-connection',{'viability':{'state':'pending'}},'connection_not_ready'),('revoked-connection',{'viability':{'state':'revoked'}},'connection_not_ready'),('degraded-route',{'viability':{'state':'parent_degraded'}},'route_unavailable'),('host-denial-before-private-state',{'host_admitted':False,'viability':{'state':'revoked'}},'not_granted')]:
 eligibility.append({'id':ident,'facts':eligbase|changes,'expected':{'eligible':not bool(error),**({'error':error} if error else {})}})
management=[]
for action,kind,extra in [('list','instance',{}),('begin_create','instance',{'profile':'fixture.user'}),('describe','connection',{'connection':'conn-a'}),('begin_repair','connection',{'connection':'conn-a','profile':'fixture.user','expected_revision':'r7'}),('revoke','connection',{'connection':'conn-a'}),('status','acquisition',{'acquisition':'acq-a'}),('protected_complete','acquisition',{'acquisition':'acq-a'})]:
 management.append({'action':action,'target':{'owning_instance':'leaf-a','kind':kind}|extra})
values=[]
def add(ident,typ,value,expected=True):values.append({'id':ident,'type':'connectors.connection_admission.'+typ,'value':value,'schema_accepts':expected})
for r in viability:add(r['id']+'-facts','ViabilityFacts',r['facts']);add(r['id']+'-decision','ViabilityDecision',r['expected'])
for r in eligibility:add(r['id']+'-facts','EligibilityFacts',r['facts']);add(r['id']+'-decision','EligibilityDecision',r['expected'])
for r in management:add(r['action'],'ManagementTarget',r['target'])
add('evidence-requirements','EvidenceRequirements',{'connection':['identity_check','scope_check'],'operations':[{'operation':'issue.create','checks':['permission_check']}]})
add('retired-global-scope-state','ConnectionState','insufficient_scope',False)
add('typo-error','EligibilityDecision',{'eligible':False,'error':'scope_insufficient'},False)
add('missing-metadata-fact','ViabilityFacts',{k:v for k,v in base.items() if k!='metadata_available'},False)
add('unknown-target-member','ManagementTarget',{'owning_instance':'leaf-a','kind':'instance','credential':'forbidden-field'},False)
results=[]
for v in values:
 schema=json.loads((root/'schema-a/schema/types'/f'{v["type"]}.schema.json').read_text());actual=jsonschema.Draft202012Validator(schema).is_valid(v['value']);assert actual==v['schema_accepts'],v['id'];results.append({'id':v['id'],'accepted':actual,'expected_matched':True})
# Illustrate the type checker's actual boundary, rather than claiming predicate execution.
contradiction={'eligible':True,'error':'not_granted'}
schema=json.loads((root/'schema-a/schema/types/connectors.connection_admission.EligibilityDecision.schema.json').read_text());assert jsonschema.Draft202012Validator(schema).is_valid(contradiction)
(out/'decision-vectors.json').write_text(json.dumps({'scope':'Declared textual expectations over trusted facts; no readiness reducer, admission checker, clock, database, provider or management implementation executed. Optional semantic fields use omission, not a wire-null codec.','viability':viability,'eligibility':eligibility,'management_targets':management,'typed_values':values},indent=2)+'\n')
(out/'type-results.json').write_text(json.dumps({'expectations':len(results),'matched':len(results),'results':results,'known_limit':{'value':contradiction,'generic_schema_accepts':True,'semantic_contract_refuses':True,'reason':'Cross-value decision predicates remain explicit UNMAPPED obligations.'}},indent=2)+'\n')
a={str(p.relative_to(root/'schema-a')):p.read_bytes() for p in (root/'schema-a').rglob('*.json')};b={str(p.relative_to(root/'schema-b')):p.read_bytes() for p in (root/'schema-b').rglob('*.json')};assert a==b
selected={k:v for k,v in a.items() if 'connectors.connection_admission.' in k}
for k,v in selected.items():p=out/k;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(v)
(out/'projection-manifest.json').write_text(json.dumps({'ess':'0.20.0','two_runs_identical':True,'all_artifacts':len(a),'selected':{k:hashlib.sha256(v).hexdigest() for k,v in selected.items()}},indent=2)+'\n')
print(json.dumps({'viability_cases':len(viability),'eligibility_cases':len(eligibility),'management_targets':len(management),'schema_expectations':len(results),'generated_artifacts':len(a),'selected_copies':len(selected)}))
```
