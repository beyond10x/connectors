# One-off specification evidence check

This retained transcript is not installed project tooling or a runtime conformance implementation. First run failed because PyYAML parsed status-code mapping keys as integers; converting those keys to strings fixed the audit. Vendor bytes were unchanged. A subsequent case label was corrected from legacy-new-error to unknown-error-code because it validates a private enum, not the legacy decoder. Final observed exit 0.

The correction recheck added four exact official source archives and fourteen version-input shape cases (twelve invalid semantic inputs still accepted by generic String, plus two admitted lexical boundaries). All 46 shape expectations passed; no provider, admission predicate or race was executed.

```python
from pathlib import Path
import json,hashlib,gzip,shutil,yaml
from jsonschema import Draft202012Validator
base=Path('docs/evidence/restart-visibility-20260908');work=Path('.local/mutation-profiles-20260908');vendor=work/'vendor';dest=base/'vendor';dest.mkdir(exist_ok=True)
sources=[]
def preserve(name,url,kind):
 data=(vendor/name).read_bytes();archive=name+'.gz';(dest/archive).write_bytes(gzip.compress(data,mtime=0));sources.append({'source':url,'kind':kind,'archive':'vendor/'+archive,'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
preserve('docker-v1.56.yaml','https://docs.docker.com/reference/api/engine/version/v1.56.yaml','official Docker v1.56 Swagger 2.0')
assert sources[-1]['sha256']=='f09a22ad220d551dca5d6c402052204d3e9fd1f16943cd78ba6677ea1266616f'
for row in json.loads((vendor/'kubernetes-source-hashes.json').read_text()):
 preserve(row['file'],row['url'],'official pinned Kubernetes documentation/source');assert sources[-1]['sha256']==row['sha256']
old=Path('.local/mutation-profiles-20260908/reviewer-b/initial/historical/crates/integration-kubernetes/src/local_workloads.rs').read_bytes();(dest/'old-kubernetes-local_workloads.rs.gz').write_bytes(gzip.compress(old,mtime=0));sources.append({'source':'../connectors@81459ac42ddd518d3942f4b079841e9e0ed6efc8:crates/integration-kubernetes/src/local_workloads.rs','kind':'pinned predecessor evidence','archive':'vendor/old-kubernetes-local_workloads.rs.gz','sha256':hashlib.sha256(old).hexdigest(),'bytes':len(old)})
supplement=Path('.local/restart-visibility-20260908/reviewer-b/recheck1')
for row in json.loads((supplement/'supplemental-provider-hashes.json').read_text()):
 data=(supplement/row['path']).read_bytes();assert len(data)==row['bytes'] and hashlib.sha256(data).hexdigest()==row['sha256']
 archive='kubernetes-v1.35.0-'+Path(row['path']).name+'.gz';(dest/archive).write_bytes(gzip.compress(data,mtime=0));sources.append({'source':row['source'],'kind':'official pinned Kubernetes conditional-update source','archive':'vendor/'+archive,'sha256':row['sha256'],'bytes':len(data)})
(base/'provider-source-hashes.json').write_text(json.dumps(sources,indent=2)+'\n')
v=yaml.safe_load((vendor/'docker-v1.56.yaml').read_text());assert v['swagger']=='2.0' and v['info']['version']=='1.56';endpoints={}
for path,method in [('/containers/json','get'),('/images/json','get'),('/volumes','get'),('/networks','get'),('/containers/{id}/json','get'),('/containers/{id}/logs','get'),('/containers/{id}/start','post'),('/containers/{id}/stop','post'),('/containers/{id}/restart','post')]:
 row=v['paths'][path][method];endpoints[path]={'method':method,'operation_id':row['operationId'],'response_codes':sorted(str(code) for code in row['responses']),'parameters':[x.get('name',x.get('$ref')) for x in row.get('parameters',[])]}
assert '304' in endpoints['/containers/{id}/start']['response_codes'] and '304' in endpoints['/containers/{id}/stop']['response_codes'] and '304' not in endpoints['/containers/{id}/restart']['response_codes'];(base/'docker-endpoint-audit.json').write_text(json.dumps(endpoints,indent=2)+'\n')
a=work/'rv-schema-a';b=work/'rv-schema-b';files=sorted(p.relative_to(a) for p in a.rglob('*') if p.is_file());assert files==sorted(p.relative_to(b) for p in b.rglob('*') if p.is_file());hashes=[]
for rel in files:
 data=(a/rel).read_bytes();assert data==(b/rel).read_bytes();hashes.append({'path':str(rel),'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
(base/'projection-hashes.json').write_text(json.dumps(hashes,indent=2)+'\n')
names=['connectors.mutations.'+x for x in ['IdempotencyKind','ContainerLifecycleAction','ContainerLifecycleIntent','DeploymentRestartTarget','PreparedDeploymentRestart']]+['connectors.declarations.OperationAvailability','connectors.declarations.AvailabilityDecision'];dest=base/'schema'/'types';dest.mkdir(parents=True,exist_ok=True)
for name in names:shutil.copyfile(a/'schema'/'types'/(name+'.schema.json'),dest/(name+'.schema.json'))
cases=[]
def case(id,type,value,accept=True,semantic=None,note='Shape only; no runtime/predicate executed.',context=None):
 cases.append({'id':id,'type':type,'value':value,'schema_accepts':accept,'semantic_accepts':semantic,'note':note,'context':context})
m='connectors.mutations.';d='connectors.declarations.'
for x in ['none','natural','keyed']:case('kind-'+x,m+'IdempotencyKind',x)
for x in ['start','stop','restart']:case('action-'+x,m+'ContainerLifecycleAction',x)
c={'instance_ref':'docker-a','connection_ref':'daemon-a','container_selector':'api','expected_container_id':'a'*64,'action':'start'}
case('start-intent',m+'ContainerLifecycleIntent',c,semantic=True)
stop={**c,'action':'stop','stop_signal':'SIGTERM','stop_timeout_seconds':10};case('stop-intent',m+'ContainerLifecycleIntent',stop,semantic=True);case('restart-intent',m+'ContainerLifecycleIntent',{**stop,'action':'restart'},semantic=True)
case('unknown-action',m+'ContainerLifecycleIntent',{**c,'action':'kill'},False)
case('missing-target',m+'ContainerLifecycleIntent',{k:v for k,v in c.items() if k!='expected_container_id'},False)
case('timeout-string',m+'ContainerLifecycleIntent',{**stop,'stop_timeout_seconds':'10'},False)
case('wait-out-of-bound',m+'ContainerLifecycleIntent',{**stop,'stop_timeout_seconds':11},True,False,'Counterexample: generic Integer does not enforce selected 0..10 wait.')
case('partial-container-id',m+'ContainerLifecycleIntent',{**c,'expected_container_id':'abc'},True,False,'Counterexample: generic String does not enforce full container identity syntax.')
case('start-with-stop-options',m+'ContainerLifecycleIntent',{**stop,'action':'start'},True,False,'Counterexample: action/option coupling is a normative predicate.')
case('stop-missing-options',m+'ContainerLifecycleIntent',{**c,'action':'stop'},True,False,'Counterexample: stop intent requires resolved fixed signal/wait despite generic Optional.')
t={'instance_ref':'kube-a','connection_ref':'cluster-a','namespace':'apps','name':'api','uid':'uid-a','resource_version':'123'}
case('restart-target',m+'DeploymentRestartTarget',t,semantic=True);case('version-not-string',m+'DeploymentRestartTarget',{**t,'resource_version':123},False);case('missing-uid',m+'DeploymentRestartTarget',{k:v for k,v in t.items() if k!='uid'},False)
for label,value in [('zero','0'),('zero-alias','00'),('all-zero','000000'),('leading-zero','00042'),('plus','+42'),('negative','-1'),('leading-space',' 42'),('trailing-newline','42\n'),('radix','0x2a'),('non-ascii','４２'),('overflow','18446744073709551616'),('empty','')]:
 case('conditional-version-'+label,m+'DeploymentRestartTarget',{**t,'resource_version':value},True,False,'Counterexample: generic String does not enforce the selected built-in Deployment canonical positive decimal 1..18446744073709551615 rule. No provider/predicate execution.')
for label,value in [('minimum','1'),('maximum','18446744073709551615')]:
 case('conditional-version-'+label,m+'DeploymentRestartTarget',{**t,'resource_version':value},semantic=True,note='Admitted lexical/range boundary only; no proof this provider object/version exists or a PATCH would succeed.')
case('prepared-patch',m+'PreparedDeploymentRestart',{'target':t,'restart_marker':'1788893000000'},semantic=True)
case('marker-number',m+'PreparedDeploymentRestart',{'target':t,'restart_marker':1788893000000},False)
case('marker-non-decimal',m+'PreparedDeploymentRestart',{'target':t,'restart_marker':'later'},True,False,'Counterexample: generic String does not prove trusted fixed epoch-ms generation.')
f={'implemented':True,'binding_supported':True,'enabled':True,'metadata_admitted':True,'lookup_admitted':True,'dependency_ready':True}
case('available-facts',d+'OperationAvailability',f,semantic=True);case('not-ready-facts',d+'OperationAvailability',{**f,'dependency_ready':False},semantic=True);case('disabled-facts',d+'OperationAvailability',{**f,'enabled':False},semantic=True)
case('facts-string-bool',d+'OperationAvailability',{**f,'enabled':'false'},False)
case('visible-decision',d+'AvailabilityDecision',{'advertised':True},semantic=True,context=f)
case('disabled-decision',d+'AvailabilityDecision',{'advertised':False,'lookup_error':'forbidden'},semantic=True,context={**f,'enabled':False})
case('disabled-advertised',d+'AvailabilityDecision',{'advertised':True},True,False,'Counterexample: schema does not compute projection from the supplied context.',{**f,'enabled':False})
case('denied-lookup-proceeds',d+'AvailabilityDecision',{'advertised':False},True,False,'Counterexample: no lookup error is not proof of current admission.',{**f,'lookup_admitted':False})
case('unknown-error-code',d+'AvailabilityDecision',{'advertised':False,'lookup_error':'disabled'},False)
case('optional-is-not-null',d+'AvailabilityDecision',{'advertised':False,'lookup_error':None},False)
results=[]
for row in cases:
 schema=json.loads((dest/(row['type']+'.schema.json')).read_text());Draft202012Validator.check_schema(schema);errors=list(Draft202012Validator(schema).iter_errors(row['value']));got=not errors;assert got==row['schema_accepts'],row['id'];results.append({'id':row['id'],'schema_accepted':got,'errors':[e.message for e in errors]})
(base/'typed-values.json').write_text(json.dumps(cases,indent=2)+'\n');(base/'shape-results.json').write_text(json.dumps(results,indent=2)+'\n')
result={'provider_sources_pinned':len(sources),'docker_endpoint_declarations_checked':len(endpoints),'schema_artifacts_compared':len(files),'copied_schemas':len(names),'shape_expectations':len(cases),'shape_negatives':sum(not x['schema_accepts'] for x in cases),'accepted_semantic_counterexamples':sum(x['schema_accepts'] and x['semantic_accepts'] is False for x in cases)};(base/'check-results.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
```

Observed output, exit 0:

```json
{
  "provider_sources_pinned": 8,
  "docker_endpoint_declarations_checked": 9,
  "schema_artifacts_compared": 222,
  "copied_schemas": 7,
  "shape_expectations": 46,
  "shape_negatives": 9,
  "accepted_semantic_counterexamples": 19
}
```
