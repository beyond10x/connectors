# Fixture generation and inspection transcript

This one-off Python verification uses cryptography Ed25519 and jsonschema against pinned ESS output. It generates public deterministic fixtures and observes primitives/type shape; it implements no Connectors receiver, wire codec, clock, store, issuer or provider. Its canonicalizer is only for the fixed fixture JSON subset. Optional semantic values deliberately omit null members to match the generic ESS projection.

```python
from pathlib import Path
import base64,copy,hashlib,json,datetime
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding,PublicFormat
from cryptography.exceptions import InvalidSignature
import jsonschema
out=Path('docs/evidence/federated-approval-20260908');out.mkdir(exist_ok=True)
schemas=Path('.local/federated-approval-20260908/schema-first/schema/types')
def canonical(x):return json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
def sha(x):return hashlib.sha256(x).hexdigest()
def b64(x):return base64.urlsafe_b64encode(x).rstrip(b'=').decode()
def semantic(x):
 if isinstance(x,dict):return {k:semantic(v) for k,v in x.items() if v is not None}
 if isinstance(x,list):return [semantic(v) for v in x]
 return x
business={'title':'fixture issue','metadata':{'z':2,'a':1}}
authority={'scope':{'tenant':'tenant-a','realm':None,'caller':'principal-a','executor':None},'current_authority':{'id':'authority-a','sha256':'a'*64},'executor':None}
route={'gateway_instance':'gateway-prod','route_id':'prod','route_revision':'route-r3'}
subject={'format':'connectors.approval-subject/v1','target':{'instance':'leaf-prod','operation':'issue.create','connection':'conn-team','connection_revision':'connection-r7','contract':'operations/v1alpha1','profile':'mutation','descriptor_revision':'leaf-d9','configuration_revision':'leaf-config4'},'authority':authority,'origin':{'kind':'federated','authority_ref':'gateway-prod'},'route':route,'canonicalization':'adapter-v1-canonical-json','input_sha256':sha(canonical(business)),'approval_mode':'required'}
subjects=[]
for id in ['federated-managed','federated-configured','direct-managed','direct-configured','federated-executor']:
 s=copy.deepcopy(subject)
 if 'configured' in id:s['target']['connection']='configured-main'
 if id.startswith('direct'):
  s['origin']={'kind':'direct','authority_ref':'leaf-prod'};s['route']=None
 if id=='direct-configured':s['authority']['scope']['tenant']=None;s['authority']['current_authority']=None
 if id=='federated-executor':
  s['authority']['scope']['executor']='agent-a';s['authority']['executor']={'agent':'agent-a','revision':'agent-r2','authority_snapshot':{'id':'executor-authority-a','sha256':'b'*64}}
 subjects.append({'id':id,'subject':s,'canonical_utf8':canonical(s).decode(),'subject_sha256':sha(canonical(s))})
issuer_seed=hashlib.sha256(b'PUBLIC F03 APPROVAL FIXTURE ONLY').digest();gateway_seed=hashlib.sha256(b'PUBLIC F03 GATEWAY FIXTURE ONLY').digest()
keys={'approval':Ed25519PrivateKey.from_private_bytes(issuer_seed),'delivery':Ed25519PrivateKey.from_private_bytes(gateway_seed)}
t=int(datetime.datetime(2026,9,8,12,tzinfo=datetime.timezone.utc).timestamp());ref='1'*64
approval_header={'alg':'Ed25519','kid':'approval-fixture-key','typ':'b10x.connectors-approval.v1+jws'}
delivery_header={'alg':'Ed25519','kid':'gateway-fixture-key','typ':'b10x.connectors-delegation.v1+jws'}
def sign(header,claims,key):
 signing=b64(canonical(header))+'.'+b64(canonical(claims));return signing+'.'+b64(key.sign(signing.encode()))
claims={'iss':'approval-issuer','aud':'leaf-approval-audience','reference':ref,'subject':subject,'iat':t,'nbf':t-5,'exp':t+295}
approval=sign(approval_header,claims,keys['approval'])
body={'version':'v1alpha2','request_id':'leaf-request-1','operation':'issue.create','revision':'leaf-d9','connection':'conn-team','input':business,'idempotency_key':'business-key-1','approval':{'reference':ref,'evidence':approval}}
proto={'iss':'gateway-prod','aud':'leaf-delegation-audience','receiver':'leaf-prod','purpose':'invoke','method':'POST','target':'/v1alpha2/invoke','body_sha256':sha(canonical(body)),'authority':authority,'route':route,'origin_request_id':'client-request-1','request_id':'leaf-request-1','operation':'issue.create','connection':'conn-team','descriptor_revision':'leaf-d9','execution_deadline_unix_ms':t*1000+20000,'iat':t,'nbf':t-5,'exp':t+25,'jti':'2'*64}
proofs=[{'id':'approval-federated-managed','kind':'approval','header':approval_header,'claims':claims,'jws':approval,'expect_signature':True,'protocol_expectation':'valid only under the selected issuer/current authority/subject/spend conditions'}]
for purpose in ['invoke','prepare','describe']:
 c=copy.deepcopy(proto);c['purpose']=purpose
 b=copy.deepcopy(body)
 if purpose=='prepare':
  b={'version':'v1alpha2','request_id':'leaf-prepare-1','operation':'host.approval.prepare','revision':'leaf-d9','connection':'conn-team','input':{'operation':'issue.create','input':business}}
  c.update(origin_request_id='client-prepare-1',request_id=b['request_id'],operation=b['operation'],jti='3'*64)
 if purpose=='describe':
  raw=b'';c.update(method='GET',target='/v1alpha2/describe',origin_request_id=None,request_id=None,operation=None,connection=None,descriptor_revision=None,jti='4'*64)
 else:raw=canonical(b)
 c['body_sha256']=sha(raw)
 proofs.append({'id':'delivery-'+purpose,'kind':'delivery','header':delivery_header,'claims':c,'body_utf8':raw.decode(),'jws':sign(delivery_header,c,keys['delivery']),'expect_signature':True,'expect_body_digest':True,'protocol_expectation':'requires current receiver/key/time/purpose/authority and unused durable nonce; crypto alone is insufficient'})
valid=proofs[1]
for id,change in [('altered-payload',lambda h,c:c.update(receiver='other-leaf')),('altered-header',lambda h,c:h.update(kid='other-key'))]:
 h=copy.deepcopy(valid['header']);c=copy.deepcopy(valid['claims']);change(h,c)
 token=b64(canonical(h))+'.'+b64(canonical(c))+'.'+valid['jws'].split('.')[2]
 proofs.append({'id':id,'kind':'delivery','header':h,'claims':c,'jws':token,'expect_signature':False,'protocol_expectation':'unauthorized; no trusted application context'})
p=copy.deepcopy(valid);p.update(id='changed-body-after-signing',body_utf8=valid['body_utf8']+' ',expect_body_digest=False,protocol_expectation='signature remains valid, but exact body binding refuses');proofs.append(p)
for id,edit in [('old-module-type',lambda h,c:h.update(typ='b10x.module-request.v1+jws')),('old-polymorphic-algorithm',lambda h,c:h.update(alg='EdDSA')),('wrong-receiver-resigned',lambda h,c:c.update(receiver='other-leaf')),('long-window-resigned',lambda h,c:c.update(exp=t+86400))]:
 h=copy.deepcopy(valid['header']);c=copy.deepcopy(valid['claims']);edit(h,c)
 proofs.append({'id':id,'kind':'delivery','header':h,'claims':c,'jws':sign(h,c,keys['delivery']),'expect_signature':True,'protocol_expectation':'cryptographically valid but the selected protocol must refuse this type/algorithm/receiver/time mismatch'})
# Sign malformed temporal claims deliberately; signature success is not admission.
for ident,kind,changes in [
 ('past-nbf-resigned','delivery',{'nbf':t-500}),
 ('wrong-profile-window-resigned','delivery',{'exp':t+295}),
 ('future-iat-resigned','delivery',{'iat':t+20,'nbf':t+15,'exp':t+45}),
 ('time-underflow-resigned','delivery',{'iat':2,'nbf':-3,'exp':27}),
 ('time-overflow-resigned','delivery',{'iat':9007199254740990,'nbf':9007199254740985,'exp':9007199254741015}),
 ('fractional-time-resigned','delivery',{'iat':t+0.5}),
 ('boolean-time-resigned','delivery',{'iat':True}),
 ('approval-long-window-resigned','approval',{'exp':t+86400}),
 ('approval-wrong-profile-window-resigned','approval',{'exp':t+25})]:
 base=proofs[0] if kind=='approval' else proofs[1]
 c=copy.deepcopy(base['claims']);c.update(changes)
 proofs.append({'id':ident,'kind':kind,'header':base['header'],'claims':c,'jws':sign(base['header'],c,keys[kind]),'expect_signature':True,'protocol_expectation':'valid signature; refuse the malformed/out-of-time proof before entry or spending at fixture clock [t-1,t+1]'})
results=[]
for p in proofs:
 parts=p['jws'].split('.');raw=base64.urlsafe_b64decode(parts[2]+'==')
 try:keys[p['kind']].public_key().verify(raw,(parts[0]+'.'+parts[1]).encode());valid_sig=True
 except InvalidSignature:valid_sig=False
 assert valid_sig==p['expect_signature'],p['id']
 r={'id':p['id'],'signature_valid':valid_sig}
 if 'body_utf8' in p:
  eq=sha(p['body_utf8'].encode())==p['claims']['body_sha256'];assert eq==p['expect_body_digest'];r['body_digest_matches']=eq
 results.append(r)
keydata={name:{'public_test_seed_hex':seed.hex(),'public_key_hex':keys[name].public_key().public_bytes(Encoding.Raw,PublicFormat.Raw).hex()} for name,seed in [('approval',issuer_seed),('delivery',gateway_seed)]}
(out/'cryptographic-vectors.json').write_text(json.dumps({'warning':'PUBLIC DETERMINISTIC TEST MATERIAL. Not real approval, credentials or authorized dispatch. Ed25519 primitive checks only; no Connectors verifier/store/provider is executed.','fixture_time':t,'keys':keydata,'proofs':proofs},indent=2)+'\n')
(out/'cryptographic-results.json').write_text(json.dumps({'checks':len(results),'passed':len(results),'results':results},indent=2)+'\n')
values=[]
for row in subjects:values.append({'id':row['id'],'type':'connectors.delegation.CanonicalApprovalSubject','wire_value':row['subject'],'semantic_value':semantic(row['subject']),'semantic_schema_accepts':True})
for p in proofs[:4]:values.append({'id':p['id'],'type':'connectors.delegation.ApprovalClaims' if p['kind']=='approval' else 'connectors.delegation.DeliveryClaims','wire_value':p['claims'],'semantic_value':semantic(p['claims']),'semantic_schema_accepts':True})
for name,value in [('missing-operation',semantic(subject)),('wrong-target-type',semantic(subject)),('unknown-subject-field',semantic(subject))]:
 if name=='missing-operation':value['target'].pop('operation')
 if name=='wrong-target-type':value['target']['connection']=3
 if name=='unknown-subject-field':value['surprise']=True
 values.append({'id':name,'type':'connectors.delegation.CanonicalApprovalSubject','semantic_value':value,'semantic_schema_accepts':False})
value_results=[]
for row in values:
 schema=json.loads((schemas/(row['type']+'.schema.json')).read_text());validator=jsonschema.Draft202012Validator(schema)
 accepted=validator.is_valid(row['semantic_value']);assert accepted==row['semantic_schema_accepts'],row['id']
 r={'id':row['id'],'semantic_schema_accepted':accepted,'expected_matched':True}
 if 'wire_value' in row:r['wire_bytes_shape_accepted_by_generic_ess_schema']=validator.is_valid(row['wire_value'])
 value_results.append(r)
(out/'subject-and-type-vectors.json').write_text(json.dumps({'scope':'Generic ESS Optional projection uses omission and rejects explicit null. semantic_value removes null optional members only to test the typed value projection. It is not the normative wire encoding.','canonical_subjects':subjects,'values':values},indent=2)+'\n')
(out/'type-projection-results.json').write_text(json.dumps({'passed':len(values),'results':value_results,'limitation':'Generic ESS JSON Schema does not implement mandatory-null wire fields, fixed proof constants, bounds, signatures or cross-field admission. Never use this generated schema as the wire codec.'},indent=2)+'\n')
# Equality/identity decisions: alter one canonical coordinate without silently rewriting approval.
axes=[]
for path,replacement in [('target.instance','leaf-other'),('target.operation','issue.delete'),('target.connection','conn-other'),('target.connection_revision','r8'),('target.contract','other/v1'),('target.profile','other'),('target.descriptor_revision','leaf-d10'),('target.configuration_revision','cfg5'),('authority.scope.caller','principal-b'),('authority.scope.realm','default'),('authority.scope.executor','agent-a'),('authority.current_authority.id','new-authority'),('origin.authority_ref','gateway-other'),('route.route_revision','route-r4'),('input_sha256','c'*64)]:
 changed=copy.deepcopy(subject);cursor=changed;parts=path.split('.')
 for part in parts[:-1]:cursor=cursor[part]
 cursor[parts[-1]]=replacement;assert canonical(changed)!=canonical(subject)
 axes.append({'changed_coordinate':path,'subject_equal':False,'expected_new_attempt':'approval_refused after current admission; live-key fingerprint changes conflict where the coordinate belongs to the fingerprint'})
(out/'subject-equality-audit.json').write_text(json.dumps({'canonicalization':'fixed fixture subset of adapter-v1-canonical-json; no general new canonicalizer implemented','base_subject_sha256':sha(canonical(subject)),'coordinate_checks':axes,'gateway_presentation_only_refresh':{'canonical_subject_unchanged':True,'requires_current_gateway_descriptor':True,'automatic_resend':False}},indent=2)+'\n')
print(json.dumps({'crypto_primitive_checks':len(results),'type_schema_expectations':len(values),'canonical_subject_axes':len(axes),'generic_projection_wire_accepts':[r.get('wire_bytes_shape_accepted_by_generic_ess_schema') for r in value_results if 'wire_bytes_shape_accepted_by_generic_ess_schema' in r]}))
```

## Time-predicate arithmetic transcript

```python
# Offline arithmetic audit of the stated predicates, not a receiver/store implementation.
from pathlib import Path
import json
out=Path('docs/evidence/federated-approval-20260908')
v=json.loads((out/'cryptographic-vectors.json').read_text());t=v['fixture_time'];maximum=9007199254740991
rows=[]
# Expected facts are explicit; crypto verification was done separately.
for proof in v['proofs']:
 if proof['id'] not in ['approval-federated-managed','delivery-invoke','long-window-resigned','past-nbf-resigned','wrong-profile-window-resigned','future-iat-resigned','time-underflow-resigned','time-overflow-resigned','fractional-time-resigned','boolean-time-resigned','approval-long-window-resigned','approval-wrong-profile-window-resigned']:continue
 c=proof['claims'];kind=proof['kind'];delta=25 if kind=='delivery' else 295
 bounded=all(type(c[k]) is int and 0<=c[k]<=maximum for k in ['iat','nbf','exp'])
 shape=bounded and c['nbf']==c['iat']-5 and c['exp']==c['iat']+delta
 valid=shape and c['nbf']<=t-1 and t+1<c['exp']
 expected=proof['id'] in ['approval-federated-managed','delivery-invoke']
 assert valid==expected,proof['id']
 rows.append({'id':proof['id'],'clock_interval':[t-1,t+1],'integer_bounds':bounded,'exact_profile_shape':shape,'time_predicate_accepts':valid,'expected_matched':True})
for ident,lower,upper,expected in [('not-before-equality',t-5,t-1,True),('before-not-before',t-6,t-2,False),('before-expiry',t+20,t+24,True),('expiry-equality',t+21,t+25,False),('after-expiry',t+22,t+26,False)]:
 accepts=t-5<=lower and upper<t+25;assert accepts==expected
 rows.append({'id':ident,'clock_interval':[lower,upper],'time_predicate_accepts':accepts,'expected_matched':True})
for ident,clock,key,projection,deadline,expected in [('post-ack-all-current',True,True,True,True,True),('post-ack-expired',False,True,True,True,False),('post-ack-key-revoked',True,False,True,True,False),('post-ack-projection-denied',True,True,False,True,False),('post-ack-clock-uncertain',False,True,True,True,False),('post-ack-deadline-exhausted',True,True,True,False,False)]:
 # Boolean truth-table check only. No timing, suspension, revocation or storage is executed.
 eligible=clock and key and projection and deadline;assert eligible==expected
 rows.append({'id':ident,'definite_nonce_consume_ack':True,'final_trusted_facts':{'time_valid':clock,'key_valid':key,'projection_admitted':projection,'deadline_remaining':deadline},'entry_eligible':eligible,'nonce_stays_consumed':True,'expected_matched':True})
(out/'time-admission-audit.json').write_text(json.dumps({'scope':'Offline integer/arithmetic and boolean truth-table audit. Declared trusted facts, no actual clock, nonce wait, policy revocation, admission, spend or provider execution.','checks':len(rows),'passed':len(rows),'cases':rows},indent=2)+'\n')
print(f'{len(rows)} arithmetic/truth-table expectations matched')
```
