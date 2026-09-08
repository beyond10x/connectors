# Final one-off evidence audit

This is the exact ignored audit source executed for this checkpoint, retained as documentation rather than installed project tooling. It compares against pinned baseline 3fc56bddb3a53a3e1320ecc2bfdf8a2a45952e23, checks archived source manifests and current approved hashes, and verifies the recorded outcomes and original ledger. Its resulting counts are in [final-audit.json](final-audit.json). It does not run provider/runtime conformance.

```console
python .local/mutation-profiles-20260908/audit-restart-visibility.py
```

Observed exit 0. Exact source:

```python
from pathlib import Path
from collections import Counter
import gzip, hashlib, io, json, re, subprocess, tarfile, yaml

BASE_COMMIT = '3fc56bddb3a53a3e1320ecc2bfdf8a2a45952e23'
base = Path('docs/evidence/restart-visibility-20260908')
records = json.loads((base / 'reviews/archive-manifest.json').read_text())
assert len(records) == 4
archived = 0
declared_sources = 0
final_sources = {}
def verify(data, row):
    assert len(data) == row['bytes'] and hashlib.sha256(data).hexdigest() == row['sha256'], row
for record in records:
    archive = (base / 'reviews' / record['archive']).read_bytes()
    assert len(archive) == record['archive_bytes'] and hashlib.sha256(archive).hexdigest() == record['archive_sha256']
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as tar:
        files = {m.name: tar.extractfile(m).read() for m in tar.getmembers() if m.isfile()}
        assert len(files) == len(record['files'])
        for row in record['files']:
            verify(files[row['path']], row)
            archived += 1
        for name in ['source-hashes.json', 'supplemental-source-hashes.json', 'projection-source-hashes.json', 'compiled-source-hashes.json', 'supplemental-provider-hashes.json']:
            if name not in files:
                continue
            doc = json.loads(files[name])
            for row in doc['files'] if isinstance(doc, dict) else doc:
                source = row.get('source', row.get('path'))
                snapshot = row.get('snapshot')
                if snapshot is None:
                    snapshot = row['path'] if row['path'] in files else 'snapshot/' + row['path']
                verify(files[snapshot], row)
                declared_sources += 1
                if name == 'source-hashes.json' and '-recheck2-' in record['review_id']:
                    verify(Path(source).read_bytes(), row)
                    if source in final_sources:
                        assert final_sources[source] == row['sha256']
                    final_sources[source] = row['sha256']
        raw = (base / 'reviews' / record['report']).read_bytes()
        assert raw == files['report.md'] and hashlib.sha256(raw).hexdigest() == record['report_sha256']
        slug = record['review_id'].split(':')[1]
        assert (Path('.engineering/planning/review-result') / (slug + '.md')).read_bytes().split(b'---\n', 2)[2] == raw
        if '-recheck2-' in record['review_id']:
            assert raw.startswith(b'approve\n') and yaml.safe_load(re.search(rb'```findings\n(.*?)\n```', raw, re.S)[1]) == []
for who in ['a', 'b']:
    raw = Path(f'docs/evidence/mutation-profiles-20260908/reviews/{who}-initial-report.md').read_bytes()
    stored = Path(f'.engineering/planning/review-result/mutation-profiles-{who}-initial-20260908.md').read_bytes().split(b'---\n', 2)[2]
    assert raw == stored

payloads = 0
for name in ['provider-source-hashes.json', 'compiled-evidence.json']:
    for row in json.loads((base / name).read_text()):
        verify(gzip.decompress((base / row['archive']).read_bytes()), row)
        payloads += 1
assert payloads == 11

changes = subprocess.check_output(['git', 'diff', '--name-only', BASE_COMMIT], text=True).splitlines()
normative_changes = [p for p in changes if p.startswith(('contracts/', 'docs/adapters/', 'ess/'))]
assert all(p in final_sources for p in normative_changes), set(normative_changes) - final_sources.keys()
assert not subprocess.check_output(['git', 'diff', '--name-only', BASE_COMMIT, '--', 'crates', 'adapters', 'spec-kinds', 'Cargo.toml', 'Cargo.lock'], text=True).strip()
assert not any(p.startswith(('crates/', 'adapters/', 'spec-kinds/')) for p in subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard'], text=True).splitlines())

type_additions = {}
for p in Path('ess').rglob('*.yaml'):
    old = yaml.safe_load(subprocess.check_output(['git', 'show', BASE_COMMIT + ':' + str(p)], text=True))
    new = yaml.safe_load(p.read_text())
    if old == new:
        continue
    assert old.keys() == new.keys()
    for key in old:
        if key != 'types':
            assert old[key] == new[key], (str(p), key)
    prior = {row['name']: row for row in old.get('types', [])}
    current = {row['name']: row for row in new.get('types', [])}
    assert all(current.get(k) == v for k, v in prior.items())
    type_additions[str(p)] = sorted(current.keys() - prior.keys())
assert len(type_additions['ess/domains/mutations.yaml']) == 5 and len(type_additions['ess/domains/declarations.yaml']) == 2
assert len(type_additions) == 2

journal = [json.loads(line) for line in Path('.engineering/planning/journal.jsonl').read_text().splitlines()]
outcomes = [r for r in journal if r.get('args', {}).get('kind') == 'review_outcome' and r['args'].get('reference', '').startswith(str(base / 'dispositions.md') + '#')]
expected = {}
for who, nums in [('a', ['03', '04', '05', '06', '07']), ('b', ['03', '04', '05', '06'])]:
    for num in nums:
        owner = 'contracts-mutation-visibility' if (who, num) in [('a', '07'), ('b', '06')] else 'contracts-restart-idempotency'
        expected[f'mp-{who}-{num}'] = (owner, f'review-result:mutation-profiles-{who}-initial-20260908')
expected.update({'rv-a-01': ('contracts-mutation-visibility', 'review-result:restart-visibility-a-recheck1-20260908'), 'rv-a-02': ('contracts-restart-idempotency', 'review-result:restart-visibility-a-recheck1-20260908'), 'rv-b-01': ('contracts-restart-idempotency', 'review-result:restart-visibility-b-recheck1-20260908')})
assert len(outcomes) == len(expected) == 12
assert Counter(r['args']['reference'].split('#')[1] for r in outcomes) == Counter(expected.keys())
for r in outcomes:
    finding = r['args']['reference'].split('#')[1]
    assert (r['id'], r['args']['review']) == expected[finding] and r['args']['outcome'] == 'fixed'
    assert '\n## ' + finding + '\n' in (base / 'dispositions.md').read_text()

ledger = Path('.engineering/planning/specification/contract-review-intake-20260908.md').read_text()
rows = [line for line in ledger.splitlines() if re.match(r'\| [FE][0-9]{2} \|', line)]
ids = [line.split('|')[1].strip() for line in rows]
assert len(rows) == len(set(ids)) == 48
assert all(re.fullmatch(r'`story:[^`]+`', row.split('|')[4].strip()) for row in rows)
fixed = sum(bool(re.search(r'\| Fixed\b', row)) for row in rows)
assert fixed == 32
statuses = Counter(yaml.safe_load(p.read_text().split('---\n', 2)[1])['status'] for p in Path('.engineering/planning/story').glob('contracts-*.md'))
assert statuses == {'implemented': 20, 'draft': 7}
for owner in ['contracts-restart-idempotency', 'contracts-mutation-visibility']:
    approvals = [r for r in journal if r.get('id') == owner and r.get('args', {}).get('kind') == 'approval']
    assert {r['args']['source'] for r in approvals} == {f'review-result:restart-visibility-{who}-recheck2-20260908' for who in ['a', 'b']}

links = 0
for p in sorted(set([Path(p) for p in normative_changes if p.endswith('.md')] + list(base.rglob('*.md')))):
    for match in re.finditer(r'\[[^\]]*\]\(([^)]+)\)', p.read_text()):
        ref = match[1].split('#')[0].strip('<>')
        if not ref or ref.startswith(('https:', 'http:', 'mailto:')):
            continue
        assert (p.parent / ref).exists() or (p.parent / ref).resolve() == (base / 'final-audit.json').resolve(), (str(p), ref)
        links += 1
validation = (base / 'planning-validation.log').read_text()
assert validation.startswith('109 file(s)') and validation.rstrip().endswith('valid')
assert '65 review(s) recorded no findings block:' in validation
result = {'base_commit': BASE_COMMIT, 'exact_raw_review_bodies': 6, 'new_review_archives': 4, 'archived_files_verified': archived, 'declared_archived_source_hashes_verified': declared_sources, 'current_final_approved_sources': len(final_sources), 'all_changed_normative_files_covered': normative_changes, 'provider_and_compiled_payloads_verified': payloads, 'individual_fixed_outcomes': len(outcomes), 'sole_owner_counts': dict(Counter(r['id'] for r in outcomes)), 'original_source_items': 48, 'original_fixed': fixed, 'original_open': 48 - fixed, 'owner_story_statuses': dict(statuses), 'new_private_types': type_additions, 'existing_entities_lifecycles_commands_unchanged': True, 'runtime_public_codec_adapter_schema_changes': False, 'local_file_links_checked': links, 'planning_artifacts_valid': 109, 'historical_prose_review_warnings': 63, 'empty_findings_final_review_warnings': 2, 'evidence_limits': 'Normative review, schema shapes, pinned source inspection and exact artifact integrity; no new runtime/provider/policy/race conformance or full-goal completion claim.'}
(base / 'final-audit.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
```
