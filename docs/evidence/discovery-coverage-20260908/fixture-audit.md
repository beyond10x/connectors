# One-off ESS type inspection

This transcript constructs declared values and checks actual generated schema shapes/projection hashes. It installs no project tool and executes no discovery, authority, publication, provider, route or composition behavior. Shape-valid semantic contradictions deliberately demonstrate the boundary.

```python
from pathlib import Path
import json,hashlib,jsonschema,shutil
root=Path('.local/discovery-coverage-20260908');out=Path('docs/evidence/discovery-coverage-20260908');values=[]
def add(ident,typ,value,accept=True,note='Declared semantic value; schema shape only.'):
 values.append(dict(id=ident,type='connectors.discovery.'+typ,value=value,schema_accepts=accept,note=note))
scope=dict(instance='kube-fixture',source_connection='conn-cluster-a',declaration='services.observe',profile='kubernetes-service-targets',scope_ref='scope-ab',selection_revision='selection-7')
add('scope-ab','CollectionScope',scope)
add('scope-a-only','CollectionScope',scope|dict(scope_ref='scope-a',selection_revision='selection-8'))
for state in ['complete','capped','denied','unavailable','not_scanned']:
 add('namespace-'+state,'PartitionCoverage',dict(id='part-b',kind='namespace',namespace='b',state=state))
add('grafana-collection','PartitionCoverage',dict(id='part-all',kind='collection',state='complete'))
add('explicit-cluster-scope','PartitionCoverage',dict(id='part-cluster',kind='cluster',state='complete'))
a=dict(id='part-a',kind='namespace',namespace='a',state='complete')
for ident,state,complete in [('complete','complete',True),('capped','capped',False),('denied','denied',False),('failed','unavailable',False),('unvisited','not_scanned',False)]:
 coverage=dict(complete=complete,partitions=[a,dict(id='part-b',kind='namespace',namespace='b',state=state)])
 add('coverage-'+ident,'CollectionCoverage',coverage)
 add('snapshot-'+ident,'SnapshotSummary',dict(scope=scope,generation=42,coverage=coverage,retention_truncated=False))
for state in ['observed','stale','withdrawn']:
 add('evidence-'+state,'ObservationEvidence',dict(state=state,last_seen_generation=41,observed_at='2026-09-08T17:00:00Z',valid_until='2026-09-08T17:05:00Z'))
pub=dict(attempt_current=True,predecessor_current=True,scope_unchanged=True,source_binding_current=True,parent_generation_current=True,authority_current=True,deadline_current=True,private_bindings_available=True)
add('publication-current','PublicationFacts',pub)
for field in pub:add('publication-'+field+'-false','PublicationFacts',pub|{field:False})
for decision in ['published','refused','unknown']:add('publication-decision-'+decision,'PublicationDecision',decision)
route=dict(expected_route_revision='route-7',old_observation_generation=41,new_observation_generation=42,observation_current=True,fixed_binding_equal=True,parent_generation_current=True,independently_admitted=True,exact_permission_current=True,child_enabled_and_not_revoked=True)
add('same-target-revalidation','RouteRevalidationFacts',route)
for field in ['observation_current','fixed_binding_equal','parent_generation_current','exact_permission_current','child_enabled_and_not_revoked']:
 add('route-'+field+'-false','RouteRevalidationFacts',route|{field:False})
for decision in ['same_binding_published','changed_binding','refused','unavailable','unknown']:add('route-decision-'+decision,'RouteRevalidationDecision',decision)
composition=dict(same_process=True,parent_port_registered=True,child_implementation_registered=True,profiles_compatible=True,parent_is_direct=True,independently_admitted=True)
add('wired-composition','CompositionFacts',composition)
for field in composition:add('composition-'+field+'-false','CompositionFacts',composition|{field:False})
add('retired-observation-state','ObservationState','ready',False)
add('unknown-partition-state','PartitionState','empty_means_complete',False)
add('public-null-not-generic-optional','PartitionCoverage',dict(id='part-all',kind='collection',namespace=None,state='complete'),False)
add('unknown-publication-member','PublicationFacts',pub|{'credential':'forbidden-field'},False)
add('wrong-generation-type','SnapshotSummary',dict(scope=scope,generation='42',coverage=dict(complete=True,partitions=[a]),retention_truncated=False),False)
add('missing-current-permission','RouteRevalidationFacts',{k:v for k,v in route.items() if k!='exact_permission_current'},False)
add('string-process-flag','CompositionFacts',composition|{'same_process':'true'},False)
# Intended shape-valid counterexamples: these demonstrate the unmapped boundary.
add('contradictory-complete-denied','CollectionCoverage',dict(complete=True,partitions=[a|{'state':'denied'}]),note='Schema accepts; collection completeness predicate refuses.')
add('missing-namespace-coordinate','PartitionCoverage',dict(id='part-b',kind='namespace',state='complete'),note='Schema accepts Optional omission; selected namespace-kind predicate requires the coordinate.')
add('negative-publication-generation','SnapshotSummary',dict(scope=scope,generation=-1,coverage=dict(complete=True,partitions=[a]),retention_truncated=False),note='Schema accepts Integer; normative generation bounds refuse.')
add('false-publisher-published-shape','PublicationDecision','published',note='This decision shape is also accepted alongside predecessor_current:false. No cross-value publication predicate or store was executed.')
results=[]
for v in values:
 schema=json.loads((root/'schema-a/schema/types'/f'{v["type"]}.schema.json').read_text());actual=jsonschema.Draft202012Validator(schema).is_valid(v['value']);assert actual==v['schema_accepts'],v['id'];results.append(dict(id=v['id'],accepted=actual,expected_matched=True))
a_hashes={str(p.relative_to(root/'schema-a')):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'schema-a').rglob('*') if p.is_file()};b_hashes={str(p.relative_to(root/'schema-b')):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'schema-b').rglob('*') if p.is_file()};assert a_hashes==b_hashes
selected=[]
for name in sorted(a_hashes):
 if '/connectors.discovery.' in name:
  dst=out/name;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(root/'schema-a'/name,dst);selected.append(name)
(out/'typed-values.json').write_text(json.dumps(dict(scope='Declared values checked for schema shape only; no coverage reducer, provider, authority, clock, store, route, composition or sequential scenario execution.',values=values),indent=2)+'\n')
(out/'type-results.json').write_text(json.dumps(dict(expected_decisions=len(results),matched=len(results),results=results),indent=2)+'\n')
(out/'projection-manifest.json').write_text(json.dumps(dict(ess='0.20.0',artifacts=len(a_hashes),two_projections_byte_identical=True,selected=selected,sha256=a_hashes),indent=2)+'\n')
(out/'fixture-audit.md').write_text('# One-off ESS type inspection\n\nThis transcript constructs declared values and checks actual generated schema shapes/projection hashes. It installs no project tool and executes no discovery, authority, publication, provider, route or composition behavior. Shape-valid semantic contradictions deliberately demonstrate the boundary.\n\n```python\n'+Path(__file__).read_text()+'```\n')
print(json.dumps(dict(type_expectations=len(values),negative_shapes=sum(not v['schema_accepts'] for v in values),schema_artifacts=len(a_hashes),selected=len(selected))))
```
