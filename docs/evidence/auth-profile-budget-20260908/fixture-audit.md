# One-off type projection inspection

This transcript checks only actual ESS-generated schema shapes and byte-identical projection. It installs no project tool and executes no semantic admission, budget, acquisition, cache or provider behavior. Four deliberately contradictory values demonstrate the limit. Optional projects to omission, not wire null.

```python
from pathlib import Path
import copy, hashlib, json, jsonschema, shutil
root=Path('.local/auth-profile-budget-20260908');out=Path('docs/evidence/auth-profile-budget-20260908');out.mkdir(parents=True,exist_ok=True)
values=[]
def add(ident,typ,value,accept=True,note='Declared value; schema shape only.'):
 values.append(dict(id=ident,type='connectors.auth_access.'+typ,value=value,schema_accepts=accept,note=note))
for x in ['loki','prometheus','alertmanager']:
 for label,mode,purpose,subject,scheme,capability in [('anonymous','direct_anonymous','anonymous','none','none','http-anonymous'),('via_parent','parent_authenticated','mediated_access','none','parent','mediated-http'),('bearer','credential','service_account','app','http_bearer','http-bearer'),('basic','credential','service_account','app','http_basic','http-basic')]:
  add(x+'-'+label,'AccessSelection',dict(profile=x+'.'+label,mode=mode,purpose=purpose,subject=subject,scheme=scheme,flow='static_config',capability=capability))
for flow in ['static_config','static_entry','oauth2_authorization_code','oauth2_client_credentials','oauth2_password','workload_identity','exec_plugin','host_issued']:
 v=dict(flow=flow,grants=[],sources=['https://vendor.example/fixture-only'])
 if flow=='oauth2_authorization_code':v.update(authorize_url='https://auth.vendor.example/authorize',token_url='https://auth.vendor.example/token',pkce='S256',registration_kind='confidential_client',callback_binding='host_callback',grants=['authorization_code'])
 if flow=='oauth2_client_credentials':v.update(token_url='https://auth.vendor.example/token',registration_kind='confidential_client',grants=['client_credentials'])
 add(flow,'AcquisitionDeclaration',v,note='Fixture source only, not vendor evidence. Reserved values type-check but cannot advertise acquisition support.' if flow in ['oauth2_password','workload_identity','exec_plugin','host_issued'] else 'Auth declaration shape; source adequacy and required-field predicates are not executed.')
def target(ns='a',resource='pods',verb='list',name='',subresource='',group='',version='v1'):
 return dict(verb=verb,api_group=group,api_version=version,resource=resource,namespace=ns,name=name,subresource=subresource)
a=target();b=target('b')
for ident,v in [('pods-a',a),('pods-b',b),('service-list',target(resource='services')),('service-proxy',target(resource='services',verb='get',name='prometheus',subresource='proxy')),('deployment',target(resource='deployments',group='apps')),('cluster-node',target(ns='',resource='nodes')),('named-pod',target(verb='get',name='api-0')),('pod-logs',target(verb='get',name='api-0',subresource='log'))]:add(ident,'PermissionTarget',v)
for ident,limits in [('default',(64,64,4)),('cache-only',(64,0,1)),('lower',(8,2,2))]:add('budget-'+ident,'PermissionBudget',dict(zip(['target_limit','call_limit','concurrency_limit'],limits))|{'deadline':'2026-09-08T15:00:15Z'})
for result in ['allowed','denied','unavailable']:
 for cached in [False,True]:add('observation-'+result+'-'+str(cached),'PermissionObservation',dict(target=a,result=result,cached=cached),note='Private observation shape only; unavailable never authorizes and cannot appear as denied in successful coverage.')
add('allowed-coverage','AuthorizationCoverage',dict(complete=True,targets=[dict(target=a,result='allowed'),dict(target=b,result='allowed')]))
add('partial-denied-coverage','AuthorizationCoverage',dict(complete=False,targets=[dict(target=a,result='allowed'),dict(target=b,result='denied')]))
add('no-unknown-success-coverage','AuthorizationCoverage',dict(complete=False,targets=[dict(target=a,result='unavailable')]),False)
add('no-secret-member','AccessSelection',values[0]['value']|{'credential':'not-allowed'},False)
add('unknown-mode','AccessMode','implicit_none',False)
add('unknown-flow','AcquisitionFlow','auto',False)
add('null-token-endpoint','AcquisitionDeclaration',dict(flow='oauth2_client_credentials',token_url=None,grants=['client_credentials'],sources=[]),False)
add('missing-exact-subresource','PermissionTarget',{k:v for k,v in a.items() if k!='subresource'},False)
add('string-budget','PermissionBudget',dict(target_limit='64',call_limit=64,concurrency_limit=4,deadline='2026-09-08T15:00:15Z'),False)
# Deliberately shape-valid but semantically refused: evidence of the checker's limit.
add('contradictory-anonymous-bearer','AccessSelection',values[0]['value']|{'scheme':'http_bearer'},note='Schema accepts; profile combination rule refuses. No semantic reducer executed.')
add('contradictory-complete-denial','AuthorizationCoverage',dict(complete=True,targets=[dict(target=a,result='denied')]),note='Schema accepts; coverage rule forbids complete:true for denial.')
add('over-ceiling-budget','PermissionBudget',dict(target_limit=65,call_limit=65,concurrency_limit=5,deadline='2026-09-08T15:00:15Z'),note='Schema accepts Integer shape; normative ceilings refuse.')
add('missing-code-endpoints','AcquisitionDeclaration',dict(flow='oauth2_authorization_code',grants=[],sources=[]),note='Schema accepts; required-flow-field/source rules refuse.')
results=[]
for v in values:
 schema=json.loads((root/'schema-a/schema/types'/f'{v["type"]}.schema.json').read_text())
 actual=jsonschema.Draft202012Validator(schema).is_valid(v['value'])
 assert actual==v['schema_accepts'],v['id']
 results.append(dict(id=v['id'],accepted=actual,expected_matched=True))
files_a={str(p.relative_to(root/'schema-a')):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'schema-a').rglob('*') if p.is_file()}
files_b={str(p.relative_to(root/'schema-b')):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'schema-b').rglob('*') if p.is_file()}
assert files_a==files_b
selected=[]
for name,digest in sorted(files_a.items()):
 if '/connectors.auth_access.' in name:
  dest=out/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(root/'schema-a'/name,dest);selected.append(name)
(out/'typed-values.json').write_text(json.dumps(dict(scope='Declared values and schema expectations only; no admission/flow/budget/provider/clock implementation executed.',values=values),indent=2)+'\n')
(out/'type-results.json').write_text(json.dumps(dict(expected_decisions=len(results),matched=len(results),results=results),indent=2)+'\n')
(out/'projection-manifest.json').write_text(json.dumps(dict(ess='0.20.0',artifacts=len(files_a),two_projections_byte_identical=True,selected=selected,sha256=files_a),indent=2)+'\n')
(out/'fixture-audit.md').write_text('# One-off type projection inspection\n\nThis transcript checks only actual ESS-generated schema shapes and byte-identical projection. It installs no project tool and executes no semantic admission, budget, acquisition, cache or provider behavior. Four deliberately contradictory values demonstrate the limit. Optional projects to omission, not wire null.\n\n```python\n'+Path(__file__).read_text()+'```\n')
print(json.dumps(dict(type_expectations=len(results),negative_shapes=sum(not v['schema_accepts'] for v in values),artifacts=len(files_a),selected=len(selected))))
```
