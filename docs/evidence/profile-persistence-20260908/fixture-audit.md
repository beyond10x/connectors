# One-off profile value and entity inventory inspection

This transcript checks projected shapes and inventories actual ESS entities. It is not an installed project tool and executes no recognizer, canonicalizer, storage, lifecycle, authority or fault scenario. An initial unmatched parenthesis in this temporary inspection script was corrected before the successful run.

```python
from pathlib import Path
import json,hashlib,shutil,yaml,jsonschema
r=Path('.local/profile-persistence-20260908');out=Path('docs/evidence/profile-persistence-20260908');values=[]
def add(ident,typ,value,accept=True,note='Shape only; no semantic predicate executed.'):
 values.append(dict(id=ident,type='connectors.discovery.'+typ,value=value,schema_accepts=accept,note=note))
src=dict(instance_id='kube-a',connection_ref='conn-a',api_origin='https://cluster.example:443',transport_policy_revision='trust-7')
for name,v in [('a',src),('other-instance',src|dict(instance_id='kube-b')),('other-connection',src|dict(connection_ref='conn-b'))]:add('source-'+name,'ConfiguredSourceIdentity',v)
binding=dict(operation_id='services.observe',profile='kubernetes-service-targets',source=src,projection_revision='meaning-2')
add('kubernetes-binding','DeclaredObservationBinding',binding)
add('grafana-binding','DeclaredObservationBinding',binding|dict(operation_id='datasources.observe',profile='grafana-datasources',source=src|dict(instance_id='grafana-a',connection_ref='conn-grafana',api_origin='https://grafana.example:443')))
for v in ['grafana-datasources','kubernetes-service-targets']:add('profile-'+v,'ObservationProfile',v)
for v in ['grafana','alertmanager','loki','prometheus','argocd']:
 add('provider-'+v,'RecognitionProvider',v);add('inferred-'+v,'ResourceRecognition',dict(provider=v,confidence='inferred'))
for v in ['declared','inferred']:add('confidence-'+v,'RecognitionConfidence',v)
for v in ['alertmanager','loki','prometheus']:add('declared-'+v,'ResourceRecognition',dict(provider=v,confidence='declared'))
inputs=[('api-name',dict(service_name='argocd-server')),('api-label',dict(service_name='platform-argocd-server',app_name='argocd-server')),('metrics',dict(service_name='argocd-server-metrics',app_name='argocd-server-metrics')),('repo',dict(service_name='argocd-repo-server')),('redis',dict(service_name='argocd-redis')),('alias-not-argo',dict(service_name='custom',legacy_app='argocd-server')),('folded',dict(service_name='custom',app_name='ARGOCD-SERVER')),('no-trimming',dict(service_name='custom',app_name=' argocd-server ')),('monitoring',dict(service_name='prod-prometheus')),('monitoring-priority',dict(service_name='grafana-prometheus')),('argo-precedes-monitoring',dict(service_name='grafana',app_name='argocd-server')),('unrecognized',dict(service_name='custom-api'))]
for ident,v in inputs:add(ident,'KubernetesRecognitionInput',v,note='Declared recognizer input; expected classification is in textual traces, not computed by this audit.')
add('unknown-profile','ObservationProfile','caller-selected',False)
add('unknown-provider','RecognitionProvider','arbitrary-proxy',False)
add('unknown-confidence','RecognitionConfidence','verified',False)
add('unknown-input-member','KubernetesRecognitionInput',dict(service_name='x',target_adapter='argocd'),False)
add('missing-source-connection','ConfiguredSourceIdentity',{k:v for k,v in src.items() if k!='connection_ref'},False)
add('missing-service-name','KubernetesRecognitionInput',dict(app_name='argocd-server'),False)
add('public-null-not-optional','KubernetesRecognitionInput',dict(service_name='x',app_name=None),False)
add('wrong-operation-profile-pair','DeclaredObservationBinding',binding|dict(profile='grafana-datasources'),note='Intentionally shape-valid; receiver declaration/source compatibility must reject this pairing.')
add('non-https-origin','ConfiguredSourceIdentity',src|dict(api_origin='http://cluster.example'),note='Intentionally shape-valid; selected canonical HTTPS origin predicate refuses.')
add('argo-declared-in-kubernetes','ResourceRecognition',dict(provider='argocd',confidence='declared'),note='Intentionally shape-valid; Kubernetes profile requires inferred, not declared, recognition.')
results=[]
for v in values:
 p=r/'schema-a/schema/types'/(v['type']+'.schema.json');schema=json.loads(p.read_text());errors=list(jsonschema.Draft202012Validator(schema).iter_errors(v['value']));actual=not errors;assert actual==v['schema_accepts'],(v['id'],[e.message for e in errors]);results.append(dict(id=v['id'],accepted=actual,expected=v['schema_accepts']))
(out/'typed-values.json').write_text(json.dumps(values,indent=2)+'\n');(out/'type-results.json').write_text(json.dumps(dict(expected_decisions=len(values),matched=len(results),negative_shapes=sum(not x['schema_accepts'] for x in values),accepted_semantic_counterexamples=3,results=results),indent=2)+'\n')
a={str(p.relative_to(r/'schema-a')):hashlib.sha256(p.read_bytes()).hexdigest() for p in (r/'schema-a').rglob('*.json')};b={str(p.relative_to(r/'schema-b')):hashlib.sha256(p.read_bytes()).hexdigest() for p in (r/'schema-b').rglob('*.json')};assert a==b;selected=sorted({v['type'] for v in values});dest=out/'schema/types';dest.mkdir(parents=True,exist_ok=True)
for name in selected:shutil.copyfile(r/'schema-a/schema/types'/(name+'.schema.json'),dest/(name+'.schema.json'))
(out/'projection-manifest.json').write_text(json.dumps(dict(ess='0.20.0',artifacts=len(a),two_projections_byte_identical=True,selected=selected,sha256=a),indent=2)+'\n')
entities=[]
for p in sorted(Path('ess/domains').glob('*.yaml')):
 d=yaml.safe_load(p.read_text())
 for e in d.get('entities',[]):entities.append(dict(source=str(p),name=e['name'],identity=e['identity'],relations=e.get('relations',[]),lifecycle=e['lifecycle']))
assert len(entities)==11
(out/'ess-entity-inventory.json').write_text(json.dumps(dict(entities=entities,entity_count=11,limitation='Declared ESS identities/relations only; no durable backend, source provenance, atomicity or live authority executed.'),indent=2)+'\n')
print(f'{len(values)}/{len(values)} schema expectations; 7 rejected shapes; 3 accepted semantic counterexamples; {len(a)} identical artifacts; {len(selected)} copied schemas; 11 existing ESS entities inventoried.')
```
