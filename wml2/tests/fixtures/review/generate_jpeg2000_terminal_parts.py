"""Import-safe last-codestream-part Psot=0 fixtures (T.800 A.4.2/Table A.6).

terminal_psot(encoded) -> (bytes, proof) changes only the final SOT Psot field.
Existing validated payloads, source files and independent PLT proofs are reused;
no encoder runs. Nonzero lengths locate the original final part, and the known
codestream EOC establishes its terminal boundary. Packet markers are never used.
Twenty-field positive and twenty-one-field negative manifests follow the existing
parts manifests; peak join budgets are unchanged. OpenJPEG validates each final
positive once. Negative structural classification is independent of whether the
oracle permissively recovers damaged files. Only one intentionally interleaved
pair is valid Part1 but outside the decoder's grouped-raster profile.
"""
import importlib.util
import json
import sys
from io import BytesIO
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('_jpeg2000_unknown_parts_fixture_tools', ROOT / 'generate_jpeg2000_unknown_parts.py')
UNKNOWN = importlib.util.module_from_spec(spec)
spec.loader.exec_module(UNKNOWN)
PARTS, VERSIONS = UNKNOWN.PARTS, UNKNOWN.VERSIONS


def terminal_psot(encoded):
    """Set only the known final part's Psot to zero, retaining final EOC."""
    headers = UNKNOWN.walk_sot(encoded)
    last = headers[-1]
    assert last['position'] + last['psot'] == len(encoded) - 2
    out = bytearray(encoded)
    position = last['position'] + 6
    out[position:position + 4] = bytes(4)
    assert out[:position] == encoded[:position] and out[position + 4:] == encoded[position + 4:]
    proof = dict(input_sha256=PARTS.sha(encoded), output_sha256=PARTS.sha(out),
                 changed_field='final SOT Psot', changed_position=position,
                 original_psot=last['psot'], final_eoc_position=len(encoded)-2,
                 codestream_offset=PARTS.stream_offset(encoded), before=headers,
                 packet_bodies_unchanged=True, encoded_length_unchanged=True)
    return bytes(out), proof


def simple_fields(name, source_name):
    """Independent SIZ/COD fields for pre-existing one-part level-zero controls."""
    data = (ROOT / name).read_bytes()
    offset = PARTS.stream_offset(data)
    stream, pos, profile = data[offset:], 2, {}
    while stream[pos:pos + 2] != bytes.fromhex('ff90'):
        size = int.from_bytes(stream[pos + 2:pos + 4], 'big')
        assert size >= 2
        b = stream[pos + 4:pos + 2 + size]
        if stream[pos:pos + 2] == bytes.fromhex('ff51'):
            xe, ye, xo, yo, tw, th, gx, gy = [int.from_bytes(b[i:i + 4], 'big') for i in range(2, 34, 4)]
            c = int.from_bytes(b[34:36], 'big')
            profile.update(w=xe-xo,h=ye-yo,tw=tw,th=th,xo=xo,yo=yo,gx=gx,gy=gy,c=c,
                           tiles=((xe-gx+tw-1)//tw)*((ye-gy+th-1)//th))
        elif stream[pos:pos + 2] == bytes.fromhex('ff52'):
            profile.update(cbw=1 << (b[6]+2),cbh=1 << (b[7]+2),levels=b[5],mct=b[4])
        pos += size + 2
    maximum, peak = UNKNOWN.validate_positive_headers(data)
    return [PARTS.sha(data),name,PARTS.sha((ROOT/source_name).read_bytes()),source_name] + [str(profile[k]) for k in ['w','h','cbw','cbh','levels','c','mct','tw','th','tiles','xo','yo','gx','gy']] + [str(maximum),str(peak)]


def oracle_status(data, fields):
    try:
        with Image.open(BytesIO(data)) as image:
            image.load()
            actual = image.tobytes()
            source = (ROOT / fields[3]).read_bytes()
            same = image.size == (int(fields[4]), int(fields[5])) and actual == source
        return 'source-exact' if same else 'decoded-different'
    except Exception as error:
        return 'rejected: ' + type(error).__name__


def generate():
    positive, negative = [VERSIONS], [VERSIONS]
    evidence = dict(tool_versions=VERSIONS, normative_source='T.800 A.4.2/Table A.6',
                    encoder_reruns=0, positive={}, negative={})
    for filename in ['jpeg2000-parts-proof.json','jpeg2000-unknown-parts-proof.json']:
        evidence[filename+'_sha256'] = PARTS.sha((ROOT / filename).read_bytes())
    base = UNKNOWN.load_manifest('jpeg2000-parts-manifest.txt')
    unknown = UNKNOWN.load_manifest('jpeg2000-unknown-parts-manifest.txt')
    selected = []
    for components, mct in [(1,0),(3,0),(3,1)]:
        stem = 'jp2_16x16_d0_noise' if components == 1 else f'jp2_rgb_16x16_d0_noise_mct{mct}'
        source = 'jp2_16x16_d0_noise.gray' if components == 1 else 'jp2_rgb_16x16_d0_noise.rgb'
        for ext in ['j2k','jp2']:
            selected.append((simple_fields(stem+'.'+ext,source), 'existing source-exact level0 control'))
    selections = [(base,'28x32_d1_',None),(unknown,'28x32_d2_','late_known'),
                  (unknown,'49x53_d3_', 'all_zero'),(base,'97x101_d4_',None),
                  (base,'33x35_d5_',None),(unknown,'1024x1024_d5_','all_zero'),
                  (base,'79x78_d1_',None),(unknown,'1x1_d5_','all_zero')]
    for manifest, geometry, suffix in selections:
        fields = [f for f in manifest if geometry in f[1] and (suffix is None or Path(f[1]).stem.endswith('_'+suffix))]
        assert len(fields) == 6, (geometry, len(fields))
        selected.extend((f, 'jpeg2000-unknown-parts-proof.json' if manifest is unknown else 'jpeg2000-parts-proof.json') for f in fields)
    for fields, proof_file in selected:
        original = (ROOT / fields[1]).read_bytes()
        assert PARTS.sha(original) == fields[0]
        data, proof = terminal_psot(original)
        name = 'jp2_terminal_' + fields[1].removeprefix('jp2_')
        UNKNOWN.oracle_for(data, fields)
        (ROOT / name).write_bytes(data)
        row = fields.copy(); row[0], row[1] = PARTS.sha(data), name
        positive.append(' '.join(row))
        evidence['positive'][name] = dict(input_file=fields[1],source_file=fields[3],source_sha256=fields[2],
                                         base_proof_file=proof_file, base_proof_key=fields[1], header_proof=proof)
    assert len(positive)-1 == 54

    def add_negative(fields, data, reason, classification, transformation):
        assert classification in ['malformed','valid-unsupported']
        name = f'unsupported_jp2_terminal_{Path(fields[1]).stem}_{reason}{Path(fields[1]).suffix}'
        status = oracle_status(data, fields)
        if classification == 'valid-unsupported':
            assert status == 'source-exact'
        (ROOT / name).write_bytes(data)
        row = fields.copy(); row[0], row[1] = PARTS.sha(data), name
        negative.append(' '.join(row[:20]+[reason]))
        evidence['negative'][name] = dict(input_file=fields[1],input_sha256=fields[0],encoded_sha256=PARTS.sha(data),
                                         source_file=fields[3],source_sha256=fields[2],reason=reason,
                                         classification=classification,oracle_status=status,transformation=transformation)

    # Last one-tile RGB/RCT packet groups: all framing defects are explicit,
    # regardless of an oracle recovering pixels from missing/bad declarations.
    manual = [f for f in base if '33x35_d5_rgb_' in f[1] and '_mct1.' in f[1]]
    assert len(manual) == 2
    for fields in manual:
        original = (ROOT/fields[1]).read_bytes()
        headers = UNKNOWN.walk_sot(original)
        data, proof = terminal_psot(original)
        first, last = headers[0]['position'], headers[-1]['position']
        offset = PARTS.stream_offset(original)
        variants = {}
        bad = bytearray(data); bad[first+6:first+10]=bytes(4)
        variants['malformed_nonterminal_zero'] = bytes(bad)
        variants['malformed_missing_EOC'] = PARTS.wrap(original, offset, data[offset:-2])
        variants['malformed_empty_terminal'] = PARTS.wrap(original, offset, data[offset:last+14]+data[-2:])
        variants['malformed_trailing'] = PARTS.wrap(original, offset, data[offset:]+bytes([0]))
        bad = bytearray(data); bad[last+11] = headers[-1]['tnsot']-1
        variants['malformed_inconsistent_total'] = bytes(bad)
        variants['malformed_missing_terminal'] = PARTS.wrap(original, offset, original[offset:last]+original[-2:])
        for reason, changed in variants.items():
            add_negative(fields, changed, reason, 'malformed', dict(terminal_header_proof=proof, defect=reason))
    prior = UNKNOWN.load_manifest('jpeg2000-parts-unsupported-manifest.txt')
    for reason, mode in [('malformed_mid_header','gray_mct0'),('malformed_mid_body','rgb_mct0'),('unsupported_interleaved','rgb_mct1')]:
        fields_list = [f for f in prior if f[20] == reason and mode in f[1]]
        assert len(fields_list) == 2
        for fields in fields_list:
            original = (ROOT/fields[1]).read_bytes()
            data, proof = terminal_psot(original)
            add_negative(fields,data,reason,'valid-unsupported' if reason=='unsupported_interleaved' else 'malformed',proof)
    assert len(negative)-1 == 18
    assert all(len(row.split())==20 for row in positive[1:])
    assert all(len(row.split())==21 for row in negative[1:])
    (ROOT/'jpeg2000-terminal-parts-manifest.txt').write_text('\n'.join(positive)+'\n')
    (ROOT/'jpeg2000-terminal-parts-unsupported-manifest.txt').write_text('\n'.join(negative)+'\n')
    (ROOT/'jpeg2000-terminal-parts-proof.json').write_text(json.dumps(evidence,indent=2)+'\n')
    print(VERSIONS,'positives:',len(positive)-1,'negatives:',len(negative)-1,'encoder reruns: 0')


if __name__ == '__main__':
    generate()
