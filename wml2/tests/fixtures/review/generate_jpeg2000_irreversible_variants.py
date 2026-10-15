"""Independent mixed/differing QCC and deep scalar-derived oracle controls.

Main fixed corpus must exist first. Tile-part bodies and pass/zero-plane tags are
preserved. Deep style1 uses an exponent no smaller than the original per-band
exponents: coded passes therefore remain legal partial passes. Changed mantissas
intentionally change reconstructed coefficients; OpenJPEG supplies new decoded
pixels, and original-source quality is separate. This does not claim equivalence
to the base image. No Rust code is imported. Headers alone change; hashes bind
original/final payloads and the actual independently reconstructed oracle.
"""
import json,sys
from copy import deepcopy
from io import BytesIO
from pathlib import Path
from PIL import Image
sys.dont_write_bytecode=True
from generate_jpeg2000_irreversible import PARTS,quality,sha
ROOT=Path(__file__).resolve().parent

def markers(data):
 offset=PARTS.stream_offset(data);stream=data[offset:];p=2;result=[]
 while stream[p:p+2]!=bytes.fromhex('ff90'):
  n=int.from_bytes(stream[p+2:p+4],'big');assert n>=2
  result.append((stream[p:p+2],p,p+n+2,stream[p+4:p+n+2]));p+=n+2
 return offset,stream,p,result

def scalar_profile(body,levels):
 style,guard=body[0]&31,body[0]>>5;words=[int.from_bytes(body[p:p+2],'big') for p in range(1,len(body),2)]
 assert len(words)==(1 if style==1 else 1+3*levels)
 result=[]
 for b in range(1+3*levels):
  word=words[0 if style==1 else b];exponent=(word>>11)-((b-1)//3 if style==1 and b else 0)
  assert exponent>=0
  mantissa=word&2047;gain=0 if b==0 else 2 if b%3==0 else 1
  result.append(dict(exponent=exponent,mantissa=mantissa,Mb=guard+exponent-1,nominal_step=2.0**(8+gain-exponent)*(1+mantissa/2048)))
 return dict(style=style,guard_bits=guard,words_hex=body.hex(),steps=result)

def generate():
 base=json.loads((ROOT/'jpeg2000-irreversible-manifest.json').read_text());cases=[];proofs={}
 for case in base['cases']:
  if case['levels'] not in [1,3,5] or case['proof_key']!=f"d{case['levels']}_c{case['components']}_m{case['mct']}_rdefault":continue
  variants=['derived_deep']
  if case['components']==3 and case['levels'] in [1,5]:variants+=['qcc_c0','qcc_c1','qcc_c2']
  original=(ROOT/case['encoded_file']).read_bytes();offset,stream,sot,entries=markers(original)
  _,start,end,qcd=next(e for e in entries if e[0]==bytes.fromhex('ff5c'))
  for variant in variants:
   row=deepcopy(case);row['framing']=variant
   if variant=='derived_deep':
    old=scalar_profile(qcd,case['levels'])
    exponent=max(b['exponent']+((i-1)//3 if i else 0) for i,b in enumerate(old['steps']))
    word=(exponent<<11)|(int.from_bytes(qcd[1:3],'big')&2047)
    quant=bytes([(qcd[0]&224)|1])+word.to_bytes(2,'big');profile=scalar_profile(quant,case['levels'])
    assert all(a['Mb']>=b['Mb'] for a,b in zip(profile['steps'],old['steps']))
    modified=stream[:start]+bytes.fromhex('ff5c')+(len(quant)+2).to_bytes(2,'big')+quant+stream[end:]
    assert modified[len(modified)-(len(stream)-sot):]==stream[sot:]
    row['quantization']['qcd']=profile
   else:
    component=int(variant[-1]);quant=bytearray(qcd)
    for p in range(1,len(quant),2):
     word=int.from_bytes(quant[p:p+2],'big');mantissa=word&2047
     mantissa=mantissa+64 if mantissa<=1983 else mantissa-64
     quant[p:p+2]=((word&0xf800)|mantissa).to_bytes(2,'big')
    extra=bytes.fromhex('ff5d')+(len(quant)+3).to_bytes(2,'big')+bytes([component])+quant
    modified=stream[:sot]+extra+stream[sot:];assert modified[sot+len(extra):]==stream[sot:]
    row['quantization']['qcc'][str(component)]=scalar_profile(quant,case['levels'])
   encoded=PARTS.wrap(original,offset,modified)
   with Image.open(BytesIO(encoded)) as decoded:
    decoded.load();assert decoded.size==(case['width'],case['height']);assert decoded.mode==('L' if case['components']==1 else 'RGB')
    expected=decoded.tobytes()
   stem=Path(case['encoded_file']).stem.replace('jp2_irreversible_','jp2_irreversible_variant_')+'_'+variant
   name=stem+Path(case['encoded_file']).suffix;oracle=stem+'.pixels'
   (ROOT/name).write_bytes(encoded);(ROOT/oracle).write_bytes(expected)
   row.update(encoded_file=name,encoded_sha256=sha(encoded),oracle_file=oracle,oracle_sha256=sha(expected),
              source_quality=quality((ROOT/case['source_file']).read_bytes(),expected),proof_key=stem)
   proofs[stem]=dict(base_file=case['encoded_file'],base_sha256=sha(original),final_sha256=sha(encoded),
                    original_tile_group_sha256=sha(stream[sot:]),tile_parts_unchanged=True,
                    header_operation=variant,expected_sha256=sha(expected),source_quality_is_not_decoder_error=True)
   cases.append(row)
 assert len(cases)==42,len(cases)
 report=dict(schema=1,comparison_policy=base['comparison_policy'],cases=cases,summary=dict(encodings=len(cases),deep_derived=18,differing_qcc=24))
 (ROOT/'jpeg2000-irreversible-variants-manifest.json').write_text(json.dumps(report,indent=2)+'\n')
 (ROOT/'jpeg2000-irreversible-variants-proof.json').write_text(json.dumps(proofs,indent=2)+'\n')
 lines=['# encoded_sha name source_sha source w h cbw cbh levels components mct tw th tiles x0 y0 gx gy parts peak_join oracle_sha oracle tolerance']
 for c in cases:
  fields=[c['encoded_sha256'],c['encoded_file'],c['source_sha256'],c['source_file'],c['width'],c['height'],*c['block'],c['levels'],c['components'],c['mct'],*c['tile'],c['tile_count'],*c['image_origin'],*c['tile_origin'],c['max_parts_per_tile'],c['peak_join_bytes'],c['oracle_sha256'],c['oracle_file'],1]
  assert len(fields)==23;lines.append(' '.join(map(str,fields)))
 (ROOT/'jpeg2000-irreversible-variants-manifest.txt').write_text('\n'.join(lines)+'\n')
 print(json.dumps(report['summary']))
if __name__=='__main__':generate()
