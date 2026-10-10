#!/usr/bin/env python3
"""Numeric Q4 diagnostics based on the recovered ifxld4.spi decoder model.

The opcode model follows the plugin disassembly documented in
q4-ifxld4-analysis.md. This tool writes JSON only and does not modify images.
"""
import argparse
import hashlib
import json
from pathlib import Path
import unittest

WIDTH = 640
PIXELS = WIDTH * 400
MAX_WORK = 0xFDF0


def stage1(data):
    if not data:
        raise ValueError('empty compressed block')
    padded = data + bytes(3)
    position, width, codes, spans = 0, 3, [], []
    while True:
        start = position
        if position + width > len(padded) * 8:
            raise ValueError('bit stream truncated')
        value = 0
        for _ in range(width):
            value = (value << 1) | ((padded[position // 8] >> (7-position % 8)) & 1)
            position += 1
        if value == 0:
            return codes, spans, position, width
        if value == 1:
            width += 1
            if width > 16:
                raise ValueError('unsupported code width')
        else:
            codes.append(value-2)
            spans.append([start, position])
            if len(codes) > MAX_WORK // 2:
                raise ValueError('dictionary stream too large')


def decompress(data):
    codes, spans, bit_end, width = stage1(data)
    if not codes or codes[0] >= 18:
        raise ValueError('invalid first dictionary code')
    entries = [bytes([i]) for i in range(18)]
    previous, current, storage_used = codes[0], 17, 0
    output, emissions, termination = bytearray(), [], None
    for index in range(1, len(codes)+1):
        next_code = codes[index] if index < len(codes) else 0
        if next_code > current:
            termination = {'code_index': index, 'code': next_code, 'current': current}
            break
        source = entries[previous]
        first = source[0] if next_code == current else entries[next_code][0]
        new = source + bytes([first])
        storage_used += len(new)
        if storage_used > MAX_WORK-18 or len(output)+len(source) > MAX_WORK:
            raise ValueError('dictionary storage/output budget exceeded')
        emissions.append({'rle_start': len(output), 'rle_end': len(output)+len(source),
                          'code_index': index-1, 'code': previous,
                          'compressed_bits': spans[index-1]})
        output.extend(source)
        if current == len(entries):
            entries.append(new)
        else:
            entries[current] = new
        if index == len(codes):
            break
        current += 1
        previous = next_code
    return bytes(output), emissions, {'codes': len(codes), 'bit_end': bit_end,
        'input_bits': len(data)*8, 'final_width': width, 'termination': termination}


def tokens(rle, expected):
    offset = written = 0
    repeat_color = 0
    result = []
    while written < expected:
        start = offset
        token = rle[offset]
        offset += 1
        high = low = None
        if token < 16:
            color, count, kind = token, 1, 'literal'
        elif token == 16:
            high = rle[offset]
            offset += 1
            if high == 0:
                color, high, low = rle[offset:offset+3]
                offset += 3
                if color >= 16:
                    raise ValueError('invalid color')
                repeat_color = color
                kind = 'explicit_color'
            else:
                color, low, kind = repeat_color, rle[offset], 'repeat_color'
                offset += 1
            count = high*17 + low
        else:
            raise ValueError('invalid token')
        if written+count > expected:
            raise ValueError('run exceeds declared block')
        result.append({'color': color, 'count': count, 'kind': kind,
                       'rle_start': start, 'rle_end': offset, 'block_pixel': written,
                       'token': token, 'high': high, 'low': low})
        written += count
    return result, offset


class ModelChecks(unittest.TestCase):
    def test_count_model(self):
        events, used = tokens(bytes([3,16,0,5,1,2]), 20)
        self.assertEqual([(e['color'],e['count']) for e in events], [(3,1),(5,19)])
        self.assertEqual(used, 6)
        self.assertEqual(tokens(bytes([16,1,2]),19)[0][0]['count'],19)
        with self.assertRaises(ValueError):
            tokens(bytes([16,1,2]),18)

    def test_repeat_uses_last_explicit_color(self):
        events, _ = tokens(bytes([16,0,5,0,2,16,1,2]), 21)
        self.assertEqual([(e['color'],e['count']) for e in events], [(5,2),(5,19)])

    def test_literal_does_not_change_repeat_color(self):
        events, _ = tokens(bytes([3,16,1,0]), 18)
        self.assertEqual([(e['color'],e['count']) for e in events], [(3,1),(0,17)])

    def test_repeat_color_resets_for_each_block(self):
        first, _ = tokens(bytes([16,0,5,0,1]), 1)
        second, _ = tokens(bytes([16,1,0]), 17)
        self.assertEqual(first[0]['color'], 5)
        self.assertEqual(second[0]['color'], 0)


def audit_file(path, trace):
    data = path.read_bytes()
    if len(data)<22 or data[11:16] != b'MAJYO':
        raise ValueError('invalid header')
    palette_end = 22+int.from_bytes(data[16:18],'little')
    if palette_end > len(data):
        raise ValueError('truncated palette payload')
    cursor, position, reports = palette_end, 0, []
    for _ in range(bool(data[4]&2)+bool(data[4]&8)):
        if cursor+6 > len(data):
            raise ValueError('truncated auxiliary header')
        cursor += 6+int.from_bytes(data[cursor:cursor+2],'little')
        if cursor > len(data):
            raise ValueError('truncated auxiliary payload')
    ranges = {k:set() for k in ['token','high','low','color']}
    while cursor<len(data):
        header = data[cursor:cursor+6]
        if len(header)!=6:
            raise ValueError('truncated block header')
        size = int.from_bytes(header[:2],'little')
        expected = 2*int.from_bytes(header[4:6],'little')
        payload = data[cursor+6:cursor+6+size]
        if len(payload)!=size or expected==0:
            raise ValueError('truncated/empty block')
        if position+expected > PIXELS:
            raise ValueError('image blocks exceed canvas')
        rle, emissions, bit_stats = decompress(payload)
        events, used = tokens(rle, expected)
        repeats, traced = [], []
        emission_index = 0
        for event in events:
            for key in ranges:
                if event[key] is not None:
                    ranges[key].add(event[key])
            start = position+event['block_pixel']
            if event['kind']=='repeat_color':
                detail = {'global_pixel': start, 'block_pixel': event['block_pixel'],
                    'color': event['color'], 'count': event['count'],
                    'first_canvas_row': start<WIDTH,
                    'first_block_row': event['block_pixel']<WIDTH,
                    'crosses_row': start % WIDTH+event['count']>WIDTH,
                    'output_end':start+event['count']}
                repeats.append(detail)
                if trace:
                    while emissions[emission_index]['rle_end']<=event['rle_start']:
                        emission_index += 1
                    related = []
                    i = emission_index
                    while i<len(emissions) and emissions[i]['rle_start']<event['rle_end']:
                        related.append(emissions[i]); i += 1
                    traced.append({**detail, 'rle_bytes':list(rle[event['rle_start']:event['rle_end']]),
                        'compressed_payload_byte':cursor+6, 'emissions':related})
        repeated = len(repeats)
        colored = sum(e['kind']=='explicit_color' for e in events)
        tail = rle[used:]
        report = {'offset':cursor,'declared_pixels':expected,
            'count_model_exact':sum(e['count'] for e in events)==expected,
            'tail_bytes':len(tail),'tail_all_zero':not any(tail),
            'repeat_color_runs':repeated,'explicit_color_runs':colored,
            'repeat_events':repeats,'bit_stats':bit_stats}
        if trace:
            report['opcode_provenance'] = traced
        reports.append(report)
        position += expected
        cursor += 6+size
    if position!=PIXELS:
        raise ValueError(f'canvas count mismatch: {position}')
    return {'file':path.name,'input_sha256':hashlib.sha256(data).hexdigest(),
        'input_bytes':len(data),'header':list(data[:22]),
        'token_ranges':{k:sorted(v) for k,v in ranges.items()},'blocks':reports}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input-dir',type=Path)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--trace',action='store_true',help='include compressed-code emission provenance')
    parser.add_argument('--self-test',action='store_true')
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(ModelChecks))
        if not result.wasSuccessful():
            raise SystemExit(1)
    if args.input_dir is None and args.output is None and args.self_test:
        return
    if args.input_dir is None or args.output is None:
        parser.error('--input-dir and --output are required for an audit')
    files = sorted(p for p in args.input_dir.iterdir() if p.is_file() and p.suffix.lower()=='.q4')
    if not files:
        parser.error('input directory has no Q4 files')
    images = [audit_file(path,args.trace) for path in files]
    report = {'status':'ifxld4_spi_rle_model_confirmed',
        'scope':'Numeric diagnostics for the ifxld4.spi RLE model; not an independent image comparison.',
        'repeat_color_state':'zero at each block; updated only by explicit-color runs',
        'trace_scope':'Emission code bits only; dictionary content depends on earlier codes.',
        'image_count':len(images),'images':images}
    args.output.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(f'Wrote numeric diagnostics for {len(images)} images to {args.output}')


if __name__=='__main__':
    main()
