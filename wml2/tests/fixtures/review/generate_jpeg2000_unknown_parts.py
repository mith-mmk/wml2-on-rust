"""Unknown/mixed TNsot fixtures; import-safe, header-only transformations.

T.800(08/2002) A.4.2/TableA.6 allows each TNsot to be zero or the correct
actual total. TPsot still starts at zero and increases per tile. Interleaving is
legal Part1, but remains outside this decoder's bounded grouped-raster profile.
https://www.itu.int/rec/dologin_pub.asp?id=T-REC-T.800-200208-S!!PDF-E&lang=e&type=items

No encoder is rerun. Validated previous multipart bytes, independent OpenJPEG
PLT boundary proofs, and raw sources are reused read-only. Only TNsot bytes
change for positives, so packet data, Psot, JP2 lengths, and join budgets stay
unchanged. Each final positive is independently compared with the source via
OpenJPEG once. Six formerly unsupported all-zero files are promoted by manifest
reference without renaming/rewriting the originals.

Positive manifest20 fields are identical to jpeg2000-parts-manifest.txt.
Negative manifest adds field21 reason. JSON records original/proof/source hashes,
all header declarations, actual part counts, payload identity, oracle outcomes.
rewrite_tnsot(encoded, mode='all_zero') -> (new_bytes, header_proof) is exported.
Modes: all_zero, late_known, known_then_unknown, alternating, known_ends.
"""
import importlib.util
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent
# AGENTS.md permits only .test* working directories; avoid __pycache__ writes.
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('_jpeg2000_parts_fixture_tools', ROOT / 'generate_jpeg2000_parts.py')
PARTS = importlib.util.module_from_spec(spec)
spec.loader.exec_module(PARTS)
VERSIONS = PARTS.VERSIONS


def walk_sot(encoded):
    """Independent SOT/Psot framing walk; never reads packet-header/entropy bits."""
    offset = PARTS.stream_offset(encoded)
    data, p, result = encoded[offset:], 2, []
    assert data[:2] == bytes.fromhex('ff4f')
    while data[p:p + 2] != bytes.fromhex('ff90'):
        n = int.from_bytes(data[p + 2:p + 4], 'big')
        assert n >= 2
        p += n + 2
    while data[p:p + 2] == bytes.fromhex('ff90'):
        assert data[p + 2:p + 4] == bytes.fromhex('000a')
        n = int.from_bytes(data[p + 6:p + 10], 'big')
        assert n > 14 and p + n <= len(data)
        assert data[p + 12:p + 14] == bytes.fromhex('ff93')
        result.append(dict(position=offset + p, isot=int.from_bytes(data[p + 4:p + 6], 'big'),
                           tpsot=data[p + 10], tnsot=data[p + 11], psot=n,
                           body_bytes=n - 14, body_sha256=PARTS.sha(data[p + 14:p + n])))
        p += n
    assert data[p:] == bytes.fromhex('ffd9')
    return result


def rewrite_tnsot(encoded, mode='all_zero'):
    assert mode in ['all_zero', 'late_known', 'known_then_unknown', 'alternating', 'known_ends']
    before = walk_sot(encoded)
    counts, seen = Counter(p['isot'] for p in before), Counter()
    out = bytearray(encoded)
    for part in before:
        isot, n = part['isot'], counts[part['isot']]
        index = seen[isot]
        seen[isot] += 1
        known = ((mode == 'late_known' and index == n - 1)
                 or (mode == 'known_then_unknown' and index == 0)
                 or (mode == 'alternating' and index % 2 == 1)
                 or (mode == 'known_ends' and index in [0, n - 1]))
        out[part['position'] + 11] = n if known else 0
    after = walk_sot(out)
    assert len(before) == len(after) and len(out) == len(encoded)
    for a, b in zip(before, after):
        assert {k: v for k, v in a.items() if k != 'tnsot'} == {k: v for k, v in b.items() if k != 'tnsot'}
    proof = dict(mode=mode, input_sha256=PARTS.sha(encoded), output_sha256=PARTS.sha(out),
                 packet_bodies_unchanged=True, actual_parts_per_tile=dict(counts), before=before, after=after)
    return bytes(out), proof


def validate_positive_headers(data):
    headers = walk_sot(data)
    counts = Counter(p['isot'] for p in headers)
    seen, previous = Counter(), 0
    for p in headers:
        isot, total = p['isot'], counts[p['isot']]
        assert 1 <= total <= 18 and p['tnsot'] in [0, total]
        assert p['tpsot'] == seen[isot]
        assert isot == previous or isot == previous + 1
        if isot != previous:
            assert seen[previous] == counts[previous]
        seen[isot] += 1
        previous = isot
    assert sorted(counts) == list(range(len(counts)))
    assert all(seen[i] == counts[i] for i in counts)
    max_parts = max(counts.values())
    peak_join = max((sum(p['body_bytes'] for p in headers if p['isot'] == isot)
                     for isot, n in counts.items() if n > 1), default=0)
    return max_parts, peak_join


def load_manifest(name):
    return [line.split() for line in (ROOT / name).read_text().splitlines()[1:]]


def oracle_for(data, fields):
    source = (ROOT / fields[3]).read_bytes()
    assert PARTS.sha(source) == fields[2]
    PARTS.oracle(data, source, int(fields[4]), int(fields[5]), int(fields[9]))


def generate():
    positive, negative, proofs, errors = [VERSIONS], [VERSIONS], {}, {}
    baseline = load_manifest('jpeg2000-parts-manifest.txt')
    old_negative = load_manifest('jpeg2000-parts-unsupported-manifest.txt')
    old_proof = ROOT / 'jpeg2000-parts-proof.json'

    def add_positive(fields, mode, *, alias=False):
        original = (ROOT / fields[1]).read_bytes()
        assert PARTS.sha(original) == fields[0]
        if alias:
            data, name = original, fields[1]
            headers = walk_sot(data)
            assert all(p['tnsot'] == 0 for p in headers)
            proof = dict(mode='promoted_existing', input_sha256=PARTS.sha(original),
                         output_sha256=PARTS.sha(data), packet_bodies_unchanged=True, after=headers)
        else:
            data, proof = rewrite_tnsot(original, mode)
            name = fields[1].replace('jp2_parts_', 'jp2_unknown_parts_', 1)
            path = Path(name)
            name = path.stem + '_' + mode + path.suffix
            (ROOT / name).write_bytes(data)
        maximum, peak = validate_positive_headers(data)
        assert (maximum, peak) == (int(fields[18]), int(fields[19]))
        oracle_for(data, fields)
        row = fields[:20]
        row[0], row[1] = PARTS.sha(data), name
        positive.append(' '.join(row))
        proofs[name] = dict(input_file=fields[1], source_file=fields[3], source_sha256=fields[2],
                            base_plt_proof_file=old_proof.name, base_proof_key=fields[1], header_proof=proof)

    for fields in baseline:
        add_positive(fields, 'all_zero')
    mixed = [a for a in baseline if a[8] == '2' and '_noise_packet_' in a[1]]
    assert len(mixed) == 6
    for mode in ['late_known', 'known_then_unknown', 'alternating', 'known_ends']:
        for fields in mixed:
            add_positive(fields, mode)
    aliases = [a for a in old_negative if a[20] == 'unsupported_unknown_TNsot']
    assert len(aliases) == 6
    for fields in aliases:
        add_positive(fields, 'all_zero', alias=True)
    assert len(positive) - 1 == 156

    def add_negative(fields, data, reason, classification):
        name = fields[1].replace('unsupported_jp2_parts_', 'unsupported_jp2_unknown_parts_', 1)
        path = Path(name)
        name = path.stem + '_unknown_' + reason + path.suffix
        (ROOT / name).write_bytes(data)
        row = fields[:20]
        row[0], row[1] = PARTS.sha(data), name
        negative.append(' '.join(row + [reason]))
        status = 'source-exact'
        try:
            oracle_for(data, fields)
        except Exception as error:
            status = 'rejected-or-different: ' + type(error).__name__
        if classification == 'valid-unsupported':
            assert status == 'source-exact'
        errors[name] = dict(input_file=fields[1], input_sha256=fields[0], encoded_sha256=PARTS.sha(data),
                            source_file=fields[3], source_sha256=fields[2], reason=reason,
                            classification=classification, oracle_status=status, headers=walk_sot(data))

    for fields in old_negative:
        if fields[20] == 'unsupported_unknown_TNsot':
            continue
        original = (ROOT / fields[1]).read_bytes()
        assert PARTS.sha(original) == fields[0]
        data, _ = rewrite_tnsot(original)
        if fields[20] == 'malformed_inconsistent_TNsot':
            # Retain the false known total; zeroing it would remove the defect.
            changed = bytearray(data)
            pos = walk_sot(data)[0]['position']
            changed[pos + 11] = original[pos + 11]
            data = bytes(changed)
        add_negative(fields, data, fields[20],
                     'valid-unsupported' if fields[20] == 'unsupported_interleaved' else 'malformed')
    for fields in aliases:
        data = (ROOT / fields[1]).read_bytes()
        headers = walk_sot(data)
        assert len(headers) > 1
        for reason, selected, tp in [('malformed_unknown_gap', headers[1], 2),
                                     ('malformed_unknown_high_tp', headers[-1], 18)]:
            changed = bytearray(data)
            changed[selected['position'] + 10] = tp
            add_negative(fields, bytes(changed), reason, 'malformed')
    assert len(negative) - 1 == 54
    assert all(len(row.split()) == 20 for row in positive[1:])
    assert all(len(row.split()) == 21 for row in negative[1:])
    (ROOT / 'jpeg2000-unknown-parts-manifest.txt').write_text('\n'.join(positive) + '\n')
    (ROOT / 'jpeg2000-unknown-parts-unsupported-manifest.txt').write_text('\n'.join(negative) + '\n')
    evidence = dict(tool_versions=VERSIONS, normative_source='T.800(08/2002) A.4.2/TableA.6',
                    base_plt_proof_sha256=PARTS.sha(old_proof.read_bytes()), positive=proofs, negative=errors)
    (ROOT / 'jpeg2000-unknown-parts-proof.json').write_text(json.dumps(evidence, indent=2) + '\n')
    print(VERSIONS, 'positives:', len(positive) - 1, 'negative:', len(negative) - 1,
          'promoted aliases:', len(aliases), 'encoder reruns: 0')


if __name__ == '__main__':
    generate()
