"""Independent 8-bit irreversible JPEG2000 fixture/oracle generation.

Import-safe. OpenJPEG encodes and reconstructs reference pixels; no production
Rust decoder is imported or executed. Sources and lossy reconstructions remain
separate, and quality errors against sources are not decoder-error assertions.
Encoder tuple parameters must be restored after JSON serialization: Pillow C
silently ignores list geometry parameters. Every SIZ/COD is checked against the
requested geometry/transform. Isolated encoder workers disable core files and
record failures; failed encodings are never admitted to the positive manifest.

JSON schema1, cases: encoded/source/oracle_file+sha256, width,height,components,
levels,mct,block,tile,tile_count,image_origin,tile_origin,max_parts_per_tile,
peak_join_bytes,framing,quantization{qcd,qcc},encoder_params,source_quality,
proof_key. Quantization steps contain exponent,mantissa,Mb and nominal_step.
Reference comparison policy is max absolute sample error1 (engineering allowance
for floating point/rounding); source quality metrics are reported independently.

Existing independent PLT tools strip PLT and split only whole packets. Unknown
counts and terminal Psot=0 alter framing only. Same-quantization QCC overrides
copy all QCD words, and scalar-derived(style1) controls use only level0 where
its single exponent/mantissa word has precisely the same meaning as style2.
No speculative deep scalar-derived rewrite is performed.

Pillow API: https://pillow.readthedocs.io/en/stable/handbook/image-file-formats.html#jpeg-2000
"""
import importlib.util
import json
import resource
import subprocess
import sys
from collections import Counter
from io import BytesIO
from pathlib import Path

from PIL import Image, __version__, features

ROOT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True

def load_helper(filename, name):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

PARTS = load_helper('generate_jpeg2000_parts.py', '_irreversible_plt')
UNKNOWN = load_helper('generate_jpeg2000_unknown_parts.py', '_irreversible_counts')
TERMINAL = load_helper('generate_jpeg2000_terminal_parts.py', '_irreversible_terminal')
VERSIONS = dict(pillow=__version__, openjpeg=features.version_codec('jpg_2000'))
sha = PARTS.sha


def quantization(encoded):
    offset = PARTS.stream_offset(encoded)
    stream, pos, result = encoded[offset:], 2, dict(qcd=None, qcc={})
    def parse(body, levels):
        style, guard = body[0] & 31, body[0] >> 5
        assert style in (1, 2) and (len(body)-1)%2==0
        words = [int.from_bytes(body[i:i+2], 'big') for i in range(1,len(body),2)]
        assert len(words) == (1 if style==1 else 1+3*levels)
        steps = []
        for index, word in enumerate(words):
            exponent, mantissa = word >> 11, word & 2047
            gain = 0 if index==0 else (2 if index%3==0 else 1)
            steps.append(dict(exponent=exponent,mantissa=mantissa,Mb=guard+exponent-1,
                              nominal_step=2.0**(8+gain-exponent)*(1.0+mantissa/2048.0)))
        return dict(style=style,guard_bits=guard,words_hex=body.hex(),steps=steps)
    levels = None
    while stream[pos:pos+2] != bytes.fromhex('ff90'):
        marker = stream[pos:pos+2]
        length = int.from_bytes(stream[pos+2:pos+4], 'big')
        assert length >= 2
        body = stream[pos+4:pos+length+2]
        if marker == bytes.fromhex('ff52'):
            levels = body[5]
        elif marker == bytes.fromhex('ff5c'):
            assert levels is not None
            result['qcd'] = parse(body,levels)
        elif marker == bytes.fromhex('ff5d'):
            result['qcc'][str(body[0])] = parse(body[1:],levels)
        pos += length+2
    assert result['qcd'] is not None
    return result


def header_variant(encoded, variant):
    """Equivalent quantization controls only; preserve every tile-part body."""
    offset = PARTS.stream_offset(encoded)
    stream, pos, qcd_pos, cod_pos = encoded[offset:], 2, None, None
    while stream[pos:pos+2] != bytes.fromhex('ff90'):
        length = int.from_bytes(stream[pos+2:pos+4], 'big')
        if stream[pos:pos+2] == bytes.fromhex('ff5c'):qcd_pos = pos
        if stream[pos:pos+2] == bytes.fromhex('ff52'):cod_pos = pos
        pos += length+2
    assert qcd_pos is not None and cod_pos is not None
    length = int.from_bytes(stream[qcd_pos+2:qcd_pos+4], 'big')
    body = stream[qcd_pos+4:qcd_pos+2+length]
    assert body[0]&31 == 2
    if variant == 'qcc_equal':
        assert int.from_bytes(stream[40:42], 'big') == 3
        extra = b''.join(bytes.fromhex('ff5d')+(len(body)+3).to_bytes(2,'big')+bytes([component])+body for component in range(3))
        modified = stream[:pos]+extra+stream[pos:]
    elif variant == 'rescaled_Mb24':
        assert stream[cod_pos+9] == 0 and len(body)==3 and body[0]>>5 == 2
        word=int.from_bytes(body[1:],'big');exponent=word>>11
        assert exponent==8
        modified=bytearray(stream)
        modified[qcd_pos+5:qcd_pos+7]=(((exponent+15)<<11)|(word&2047)).to_bytes(2,'big')
    elif variant == 'style1_d0':
        assert stream[cod_pos+9] == 0 and len(body)==3
        modified = bytearray(stream)
        modified[qcd_pos+4] = (body[0]&0xe0)|1
    else:raise ValueError(variant)
    out = PARTS.wrap(encoded,offset,modified)
    before, after = UNKNOWN.walk_sot(encoded), UNKNOWN.walk_sot(out)
    assert [p['body_sha256'] for p in before] == [p['body_sha256'] for p in after]
    return out, dict(variant=variant,input_sha256=PARTS.sha(encoded),output_sha256=PARTS.sha(out),
                     all_packet_bodies_unchanged=True,level0_equivalent_word=variant=='style1_d0',precision_rescaling_exponent_delta=15 if variant=='rescaled_Mb24' else 0)


def encode_reference(request):
    """Worker-only encoder/oracle; source stored separately from reconstruction."""
    params = dict(request['encoder_params'])
    for key in ['offset','tile_offset','tile_size','codeblock_size']:
        params[key] = tuple(params[key])
    raw = Path(request['source_path']).read_bytes()
    mode = 'L' if request['components']==1 else 'RGB'
    output = BytesIO()
    Image.frombytes(mode,tuple(request['size']),raw).save(output,format='JPEG2000',**params)
    encoded = output.getvalue()
    _,_,profile,_ = PARTS.read_plt(encoded)
    assert (profile['xo'],profile['yo']) == params['offset']
    assert (profile['gx'],profile['gy']) == params['tile_offset']
    assert (profile['tw'],profile['th']) == params['tile_size']
    assert (profile['cbw'],profile['cbh']) == params['codeblock_size']
    assert profile['levels'] == params['num_resolutions']-1
    assert profile['components'] == request['components'] and profile['mct']==params['mct']
    with Image.open(BytesIO(encoded)) as oracle:
        oracle.load()
        assert oracle.mode==mode and oracle.size==tuple(request['size'])
        reconstructed = oracle.tobytes()
    Path(request['encoded_path']).write_bytes(encoded)
    Path(request['oracle_path']).write_bytes(reconstructed)
    return dict(status='ok',encoded_sha256=PARTS.sha(encoded),oracle_sha256=PARTS.sha(reconstructed),
                profile=profile,source_quality=quality(raw,reconstructed))


def quality(source, expected):
    assert len(source)==len(expected)
    errors = [abs(a-b) for a,b in zip(source,expected)]
    return dict(max_abs_error=max(errors),mean_abs_error=sum(errors)/len(errors),different_bytes=sum(e!=0 for e in errors))


def compare_oracle(data, expected, size, components):
    with Image.open(BytesIO(data)) as image:
        image.load()
        assert image.size == tuple(size) and image.mode == ('L' if components==1 else 'RGB')
        assert image.tobytes() == expected


ceildiv = lambda a,b: -(-a//b)

class Bits:
 def __init__(self,data):self.data=data;self.p=0;self.left=0;self.last=0
 def bit(self):
  if not self.left:
   v=self.data[self.p];self.p+=1;self.left=7 if self.last==255 else 8
   if self.last==255:assert v<128
   self.last=v
  self.left-=1;return (self.last>>self.left)&1
 def number(self,n):
  v=0
  for _ in range(n):v=v*2+self.bit()
  return v
 def align(self):
  self.left=0
  if self.last==255:assert self.data[self.p]<128;self.p+=1
 def passes(self):
  if not self.bit():return 1
  if not self.bit():return 2
  v=self.number(2)
  if v!=3:return 3+v
  v=self.number(5)
  return 6+v if v!=31 else 37+self.number(7)

class Tree:
 def __init__(self,w,h):
  self.levels=[]
  while True:
   self.levels.append((w,h,[[0,255] for _ in range(w*h)]))
   if w==h==1:break
   w,h=ceildiv(w,2),ceildiv(h,2)
 def below(self,x,y,t,b):
  inherited=0
  for d in range(len(self.levels)-1,-1,-1):
   w,h,nodes=self.levels[d];node=nodes[(y>>d)*w+(x>>d)];node[0]=max(node[0],inherited)
   while node[0]<t and node[0]<node[1]:
    if b.bit():node[1]=node[0]
    else:node[0]+=1
   inherited=min(node)
  w,h,nodes=self.levels[0];return nodes[y*w+x][1]<t


def offset(encoded):
 if encoded[:2]==b'\xffO':return 0
 p=0
 while p<len(encoded):
  n=int.from_bytes(encoded[p:p+4],'big');assert n>=8
  if encoded[p+4:p+8]==b'jp2c':return p+8
  p+=n
 raise ValueError('no JP2 codestream')


def qinfo(body):
 style=body[0]&31;guard=body[0]>>5
 if style==0:steps=[dict(exponent=b>>3,mantissa=0) for b in body[1:]]
 else:
  assert (len(body)-1)%2==0
  steps=[dict(exponent=(n:=int.from_bytes(body[i:i+2],'big'))>>11,mantissa=n&2047) for i in range(1,len(body),2)]
 return dict(style=style,guard_bits=guard,steps=steps,hex=body.hex())


def packet_pass_probe(data):
 start=offset(data);s=data[start:];p=2;main=[];profile={};quant=None;qcc={}
 while s[p:p+2]!=b'\xff\x90':
  marker=s[p:p+2].hex();n=int.from_bytes(s[p+2:p+4],'big');assert n>=2
  b=s[p+4:p+n+2];main.append(dict(marker=marker,body_hex=b.hex()))
  if marker=='ff51':
   xe,ye,xo,yo,tw,th,gx,gy=[int.from_bytes(b[i:i+4],'big') for i in range(2,34,4)]
   c=int.from_bytes(b[34:36],'big');profile.update(xe=xe,ye=ye,xo=xo,yo=yo,tw=tw,th=th,gx=gx,gy=gy,components=c,precision=[(v&127)+1 for v in b[36::3]])
  elif marker=='ff52':
   profile.update(style=b[0],progression=b[1],layers=int.from_bytes(b[2:4],'big'),mct=b[4],levels=b[5],cbw=1<<(b[6]+2),cbh=1<<(b[7]+2),block_style=b[8],transform=b[9])
  elif marker=='ff5c':quant=qinfo(b)
  elif marker=='ff5d':qcc[b[0]]=qinfo(b[1:])
  p+=n+2
 nx=ceildiv(profile['xe']-profile['gx'],profile['tw']);ny=ceildiv(profile['ye']-profile['gy'],profile['th']);tiles=[];blocks=[]
 assert profile['layers']==1 and profile['style']==0 and profile['progression']==0 and profile['block_style']==0
 while s[p:p+2]==b'\xff\x90':
  assert s[p+2:p+4]==bytes([0,10]);isot=int.from_bytes(s[p+4:p+6],'big');end=p+int.from_bytes(s[p+6:p+10],'big');q=p+12;lengths=[];z=0
  while s[q:q+2]!=b'\xff\x93':
   assert s[q:q+2]==b'\xff\x58';n=int.from_bytes(s[q+2:q+4],'big');b=s[q+4:q+2+n];assert b[0]==z;z+=1;v=0
   for b in b[1:]:
    v=(v<<7)|(b&127)
    if not b&128:lengths.append(v);v=0
   assert v==0;q+=n+2
  payload=s[q+2:end];assert sum(lengths)==len(payload)
  x0=max(profile['gx']+isot%nx*profile['tw'],profile['xo']);y0=max(profile['gy']+isot//nx*profile['th'],profile['yo']);x1=min(profile['gx']+(isot%nx+1)*profile['tw'],profile['xe']);y1=min(profile['gy']+(isot//nx+1)*profile['th'],profile['ye'])
  packet_i=0;cursor=0
  for resolution in range(profile['levels']+1):
   f=1<<(profile['levels']-resolution)
   if ceildiv(x1,f)==ceildiv(x0,f) or ceildiv(y1,f)==ceildiv(y0,f):continue
   depth=profile['levels'] if resolution==0 else profile['levels']-resolution+1
   orientations=[0] if resolution==0 else [1,2,3]
   for component in range(profile['components']):
    packet=payload[cursor:cursor+lengths[packet_i]];bits=Bits(packet);length=0
    if bits.bit():
     for orientation in orientations:
      divisor=1<<depth;half=0 if orientation==0 else divisor//2
      hx=half if orientation in [1,3] else 0;hy=half if orientation in [2,3] else 0
      bx0,bx1=ceildiv(x0-hx,divisor),ceildiv(x1-hx,divisor);by0,by1=ceildiv(y0-hy,divisor),ceildiv(y1-hy,divisor)
      if bx0==bx1 or by0==by1:continue
      cbw,cbh=profile['cbw'],profile['cbh'];block_x0,block_y0=bx0//cbw,by0//cbh;block_x1,block_y1=ceildiv(bx1,cbw),ceildiv(by1,cbh);w,h=block_x1-block_x0,block_y1-block_y0
      inc,zero=Tree(w,h),Tree(w,h)
      band=0 if resolution==0 else 1+(resolution-1)*3+orientation-1
      component_q=qcc.get(component,quant);step=component_q['steps'][band];mb=component_q['guard_bits']+step['exponent']-1
      for y in range(h):
       for x in range(w):
        if not inc.below(x,y,1,bits):continue
        zbp=0
        while not zero.below(x,y,zbp+1,bits):zbp+=1;assert zbp<=mb+1
        planes=mb-zbp;passes=bits.passes();lblock=3
        while bits.bit():lblock+=1;assert lblock<=32
        n=bits.number(lblock+passes.bit_length()-1);length+=n
        blocks.append(dict(tile=isot,resolution=resolution,component=component,band=band,orientation=orientation,zbp=zbp,mb=mb,planes=planes,passes=passes,full_passes=planes*3-2,segment_bytes=n,partial=passes!=planes*3-2,passes_mod3=passes%3))
    bits.align();assert bits.p+length==len(packet),(isot,resolution,component,bits.p,length,len(packet));packet_i+=1;cursor+=len(packet)
  assert packet_i==len(lengths) and cursor==len(payload)
  tiles.append(dict(isot=isot,origin=[x0,y0],end=[x1,y1],packet_lengths=lengths,payload_sha256=sha(payload)))
  p=end
 assert s[p:]==b'\xff\xd9'
 return dict(profile=profile,main_header_markers=main,qcd=quant,qcc=qcc,tiles=tiles,blocks=blocks,included_blocks=len(blocks),partial_blocks=sum(b['partial'] for b in blocks),passes_mod3=dict(Counter(b['passes_mod3'] for b in blocks)))



def generate(work):
    assert work.name.startswith('.test')
    work.mkdir(parents=True,exist_ok=True)
    manifest = dict(schema=1,tool_versions=VERSIONS,reference='OpenJPEG reconstructed8bit samples',
                    comparison_policy=dict(max_abs_error=1,reason='floating-point/rounding engineering allowance'),
                    source_quality_is_decoder_error=False,cases=[])
    proofs = dict(schema=1,tool_versions=VERSIONS,encoder_api='PIL.Image.frombytes(...).save(BytesIO(), format="JPEG2000", **encoder_params)',
                  normative_sources=['Pillow JPEG2000 save API','T.800 A.6.4/QCD-QCC, B.11/packet tile-parts'],
                  helpers_sha256={name:PARTS.sha((ROOT/name).read_bytes()) for name in ['generate_jpeg2000_parts.py','generate_jpeg2000_unknown_parts.py','generate_jpeg2000_terminal_parts.py']},
                  encodings={},variants={},encoder_failures=[],excluded_geometry=set())
    geometries = [
        ('d0',(17,19),0,(0,0),(0,0),(32,32),(4,4),'noise'),
        ('d1',(31,33),1,(5,7),(2,3),(64,64),(16,16),'ramp'),
        ('d2',(35,37),2,(7,11),(5,9),(64,64),(4,64),'mixed'),
        ('d3',(65,67),3,(3,5),(1,2),(96,96),(64,4),'noise'),
        ('d4',(97,101),4,(5,7),(2,3),(128,128),(32,32),'ramp'),
        ('d5',(33,35),5,(7,11),(5,9),(64,64),(64,64),'noise'),
        ('canvas',(1024,1024),5,(37,41),(5,9),(256,256),(64,64),'ramp'),
        ('tiles256',(79,78),1,(101,205),(100,203),(5,5),(16,16),'mixed'),
        ('tiny',(1,1),5,(31,47),(30,46),(32,32),(4,4),'flat'),
        ('d5tiles',(64,64),5,(0,0),(0,0),(32,32),(32,32),'ramp'),
        ('wideblock',(65,67),0,(3,5),(1,2),(128,128),(1024,4),'noise')]
    rows = {}
    for tag,size,levels,origin,grid,tile,block,pattern in geometries:
        rates = [None,2,4,10] if tag in ('d0','d1','d2','d3','d4','d5') else [4 if tag=='canvas' else None]
        for components,mct in [(1,0),(3,0),(3,1)]:
            source_name = f'jp2_irreversible_{tag}_c{components}_{pattern}.source'
            source = PARTS.pixels(*size,components,pattern)
            (ROOT/source_name).write_bytes(source)
            for rate in rates:
                base = f'{tag}_c{components}_m{mct}_r{rate or "default"}'
                oracle_name = f'oracle_irreversible_{base}.pixels'
                params = dict(no_jp2=True,offset=origin,tile_offset=grid,tile_size=tile,
                              num_resolutions=levels+1,codeblock_size=block,irreversible=True,mct=mct,
                              plt=True,quality_mode='rates',progression='LRCP')
                if rate is not None:params['quality_layers']=[rate]
                request = dict(components=components,size=size,encoder_params=params,source_path=str(ROOT/source_name),
                               encoded_path=str(work/(base+'.plt.j2k')),oracle_path=str(ROOT/oracle_name))
                request_path,result_path = work/(base+'.request.json'),work/(base+'.result.json')
                request_path.write_text(json.dumps(request))
                process = subprocess.run([sys.executable,str(Path(__file__).resolve()),'--worker',str(request_path),str(result_path)],cwd=work,capture_output=True)
                stderr = process.stderr.decode('utf8','replace')
                (work/(base+'.stderr')).write_text(stderr)
                if process.returncode!=0:
                    proofs['encoder_failures'].append(dict(case=base,request=request,status='encoder-process-failure',exit_code=process.returncode,error=stderr))
                    proofs['excluded_geometry'].add(tag)
                    (work/'encoder-failures.json').write_text(json.dumps(dict(versions=VERSIONS,failures=proofs['encoder_failures']),indent=2)+'\n')
                    raise RuntimeError(f'Fixture geometry {base} failed; inspect {work/(base+".stderr")}. Do not admit it to positives.')
                result = json.loads(result_path.read_text())
                original = Path(request['encoded_path']).read_bytes()
                oracle = (ROOT/oracle_name).read_bytes()
                data, proof = PARTS.split_plt(original,'single')
                proof.update(encoder_params=params,requested_geometry_checked=True,source_sha256=PARTS.sha(source),oracle_sha256=PARTS.sha(oracle))
                proofs['encodings'][base] = proof
                profile = proof['profile']
                for ext in ['j2k','jp2']:
                    if ext=='j2k':encoded=data
                    else:
                        # Canonical enumerated-color JP2 wrapper uses an independent
                        # encoder-produced JP2 header, not a Rust container writer.
                        jp2_params=dict(params);jp2_params['no_jp2']=False
                        output=BytesIO();Image.frombytes('L' if components==1 else 'RGB',size,source).save(output,format='JPEG2000',**jp2_params)
                        wrapped=output.getvalue();offset=PARTS.stream_offset(wrapped)
                        _,_,jp2_profile,_=PARTS.read_plt(wrapped);assert jp2_profile==profile
                        encoded=PARTS.wrap(wrapped,offset,data)
                    compare_oracle(encoded,oracle,size,components)
                    name=f'jp2_irreversible_{base}.{ext}'
                    (ROOT/name).write_bytes(encoded)
                    row=dict(encoded_file=name,encoded_sha256=PARTS.sha(encoded),source_file=source_name,source_sha256=PARTS.sha(source),
                             oracle_file=oracle_name,oracle_sha256=PARTS.sha(oracle),width=size[0],height=size[1],components=components,
                             levels=levels,mct=mct,transform='irreversible9/7',block=list(block),tile=list(tile),tile_count=profile['tilecount'],
                             image_origin=list(origin),tile_origin=list(grid),max_parts_per_tile=1,peak_join_bytes=0,framing='single_known',
                             quantization=quantization(encoded),encoder_params=params,source_quality=result['source_quality'],proof_key=base)
                    manifest['cases'].append(row);rows[(tag,components,mct,rate,ext)] = row
    def derived(row,data,label,proof):
        name=Path(row['encoded_file']);name=name.stem+'_'+label+name.suffix
        expected=(ROOT/row['oracle_file']).read_bytes()
        compare_oracle(data,expected,(row['width'],row['height']),row['components'])
        (ROOT/name).write_bytes(data)
        new=dict(row);new.update(encoded_file=name,encoded_sha256=PARTS.sha(data),framing=label,quantization=quantization(data),proof_key=name)
        if label.startswith('multipart'):
            maximum,peak=proof['multipart_bounds'];new.update(max_parts_per_tile=maximum,peak_join_bytes=peak)
        proofs['variants'][name] = proof
        manifest['cases'].append(new)
        return new
    for components,mct in [(1,0),(3,0),(3,1)]:
        base=f'd5_c{components}_m{mct}_r2'
        original=(work/(base+'.plt.j2k')).read_bytes()
        multipart, plt_proof=PARTS.split_plt(original,'packet')
        for ext in ['j2k','jp2']:
            row=rows[('d5',components,mct,2,ext)]
            encoded=(ROOT/row['encoded_file']).read_bytes();offset=PARTS.stream_offset(encoded)
            known=PARTS.wrap(encoded,offset,multipart)
            maximum,peak=UNKNOWN.validate_positive_headers(known)
            bounds=[maximum,peak]
            derived(row,known,'multipart_known',dict(plt=plt_proof,multipart_bounds=bounds))
            unknown,count_proof=UNKNOWN.rewrite_tnsot(known)
            derived(row,unknown,'multipart_unknown',dict(plt=plt_proof,counts=count_proof,multipart_bounds=bounds))
            terminal,terminal_proof=TERMINAL.terminal_psot(encoded)
            derived(row,terminal,'terminal_zero',dict(terminal=terminal_proof))
            terminal,terminal_proof=TERMINAL.terminal_psot(unknown)
            derived(row,terminal,'multipart_unknown_terminal_zero',dict(plt=plt_proof,counts=count_proof,terminal=terminal_proof,multipart_bounds=bounds))
    for components,mct in [(3,0),(3,1)]:
        for ext in ['j2k','jp2']:
            row=rows[('d2',components,mct,4,ext)]
            data, proof=header_variant((ROOT/row['encoded_file']).read_bytes(),'qcc_equal')
            derived(row,data,'qcc_equal',proof)
    for components,mct in [(1,0),(3,0),(3,1)]:
        for ext in ['j2k','jp2']:
            row=rows[('d0',components,mct,4,ext)]
            data, proof=header_variant((ROOT/row['encoded_file']).read_bytes(),'style1_d0')
            derived(row,data,'style1_d0',proof)
    for components,mct in [(1,0),(3,0),(3,1)]:
        for ext in ['j2k','jp2']:
            row=rows[('d0',components,mct,None,ext)]
            data,proof=header_variant((ROOT/row['encoded_file']).read_bytes(),'rescaled_Mb24')
            derived(row,data,'rescaled_Mb24',proof)
    proofs['excluded_geometry']=sorted(proofs['excluded_geometry'])
    assert len(manifest['cases'])==214
    add_rejection_and_pass_evidence(manifest,proofs,work)
    manifest['summary']=dict(encodings=len(manifest['cases']),modes=dict(Counter(f"{r['components']}c_mct{r['mct']}" for r in manifest['cases'])),
                            levels=dict(Counter(r['levels'] for r in manifest['cases'])),framing=dict(Counter(r['framing'] for r in manifest['cases'])),
                            max_parts=max(r['max_parts_per_tile'] for r in manifest['cases']),max_tiles=max(r['tile_count'] for r in manifest['cases']),
                            oracle_source_quality='Separate lossy quality metrics, not decoder mismatch',encoder_failures=len(proofs['encoder_failures']),rejected_inputs=len(manifest['rejected']))
    (ROOT/'jpeg2000-irreversible-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    (ROOT/'jpeg2000-irreversible-proof.json').write_text(json.dumps(proofs,indent=2)+'\n')
    write_tsv(manifest)
    print(json.dumps(manifest['summary'],indent=2))


def main_marker(encoded, wanted):
    offset=PARTS.stream_offset(encoded);stream=encoded[offset:];pos=2
    while stream[pos:pos+2]!=bytes.fromhex('ff90'):
        length=int.from_bytes(stream[pos+2:pos+4],'big')
        if stream[pos:pos+2]==bytes.fromhex(wanted):return offset,pos,length,stream[pos+4:pos+2+length]
        pos+=length+2
    raise ValueError(wanted)


def replace_main_marker(encoded,wanted,body):
    offset,pos,length,_=main_marker(encoded,wanted);stream=encoded[offset:]
    segment=b'' if body is None else bytes.fromhex(wanted)+(len(body)+2).to_bytes(2,'big')+body
    return PARTS.wrap(encoded,offset,stream[:pos]+segment+stream[pos+length+2:])


def insert_main_marker(encoded,kind,body):
    offset=PARTS.stream_offset(encoded);headers=UNKNOWN.walk_sot(encoded);start=headers[0]['position']-offset;stream=encoded[offset:]
    segment=bytes.fromhex(kind)+(len(body)+2).to_bytes(2,'big')+body
    return PARTS.wrap(encoded,offset,stream[:start]+segment+stream[start:])


def negative_oracle(request):
    try:
        with Image.open(BytesIO(Path(request['encoded_path']).read_bytes())) as oracle:
            oracle.load();actual=oracle.tobytes()
            expected=Path(request['oracle_path']).read_bytes()
            return dict(status='decoded',reference_equal=actual==expected,mode=oracle.mode,size=list(oracle.size),pixel_sha256=PARTS.sha(actual))
    except Exception as error:return dict(status='rejected',error_type=type(error).__name__,error=str(error))


def add_rejection_and_pass_evidence(manifest,proofs,work):
    rows={r['encoded_file']:r for r in manifest['cases']}
    stage_counts=Counter();possible=[]
    for key,proof in proofs['encodings'].items():
        probe=packet_pass_probe((work/(key+'.plt.j2k')).read_bytes())
        included=probe['blocks'];stages=Counter({})
        for block in included:
            stage={1:'cleanup',2:'significance-propagation',0:'magnitude-refinement'}[block['passes_mod3']]
            stages[stage]+=1;stage_counts[stage]+=1
        varied=any(block['partial'] and block['passes_mod3'] in (0,2) for block in included)
        proof['packet_pass_evidence']=dict(included_blocks=len(included),partial_blocks=probe['partial_blocks'],terminal_stages=dict(stages),blocks=included,
                                         per_coefficient_last_plane_can_differ=varied,
                                         interpretation='Partial propagation/refinement can advance different coefficient subsets; this header-only proof identifies candidate cases, not actual per-coefficient MQ states.')
        if varied:possible.append(key)
    proofs['partial_pass_summary']=dict(terminal_stages=dict(stage_counts),per_coefficient_precision_candidate_cases=possible)
    manifest['rejected']=[]
    rejection_proofs={}
    for ext in ['j2k','jp2']:
        gray=rows['jp2_irreversible_d5_c1_m0_r2.'+ext]
        rgb=rows['jp2_irreversible_d2_c3_m1_r4.'+ext]
        gc=(ROOT/gray['encoded_file']).read_bytes();rc=(ROOT/rgb['encoded_file']).read_bytes()
        _,_,_,gq=main_marker(gc,'ff5c');_,_,_,rq=main_marker(rc,'ff5c')
        _,_,_,cod=main_marker(rc,'ff52')
        cases=[]
        def add(base,data,reason,classification,note):cases.append((base,data,reason,classification,note))
        add(gray,replace_main_marker(gc,'ff5c',gq[:-1]),'qcd_short','malformed','Style2 requires complete exponent/mantissa word per band.')
        add(gray,replace_main_marker(gc,'ff5c',gq+bytes(2)),'qcd_extra_band','malformed','Extra band word disagrees with COD levels.')
        add(gray,replace_main_marker(gc,'ff5c',bytes([(gq[0]&0xe0)|3])+gq[1:]),'qcd_reserved_style3','malformed','Quantization style3 is reserved.')
        add(gray,replace_main_marker(gc,'ff5c',bytes([2])+bytes(len(gq)-1)),'qcd_nonpositive_Mb','malformed','Guard0/exponent0 cannot describe included positive bit-plane blocks.')
        add(gray,replace_main_marker(gc,'ff5c',bytes([gq[0]])+bytes.fromhex('c000')*((len(gq)-1)//2)),'qcd_precision_over24','valid_unsupported','Guard2+exponent24 yields Mb25, representable Part1 but beyond bounded coefficient precision24.')
        add(gray,replace_main_marker(gc,'ff5c',bytes([(gq[0]&0xe0)|1])+bytes(2)),'scalar_derived_eps_underflow','malformed','Deep scalar-derived base exponent0 underflows derived band exponents.')
        qcc=(ROOT/('jp2_irreversible_d2_c3_m1_r4_qcc_equal.'+ext)).read_bytes()
        add(rgb,insert_main_marker(qcc,'ff5d',bytes([0])+rq),'duplicate_QCC','malformed','Component0 QCC already exists.')
        add(rgb,insert_main_marker(rc,'ff5d',bytes([3])+rq),'QCC_component3','malformed','Csiz3 component index3 lies outside0..2.')
        add(gray,insert_main_marker(gc,'ff5d',bytes([0])+gq),'gray_QCC','valid_unsupported','Same default quantization, one-component QCC legal but outside current bounded profile.')
        request=json.loads((work/'d2_c3_m1_r4.request.json').read_text());raw_multilayer=work/'two-layers.j2k'
        if not raw_multilayer.exists():
            proc=subprocess.run([sys.executable,str(Path(__file__).resolve()),'--multilayer',str(work/'d2_c3_m1_r4.request.json'),str(raw_multilayer)],cwd=work,capture_output=True)
            assert proc.returncode==0,proc.stderr
        original_multi=raw_multilayer.read_bytes();multi,_=PARTS.wrap(original_multi,0,original_multi),None
        # Remove PLT markers independently by preserving encoder packet payloads;
        # this negative tests layers, so PLT itself must not mask that feature.
        mo=PARTS.stream_offset(multi);ms=multi[mo:];mp=2
        while ms[mp:mp+2]!=bytes.fromhex('ff90'):mp+=int.from_bytes(ms[mp+2:mp+4],'big')+2
        head=ms[:mp];out=bytearray(head)
        while ms[mp:mp+2]==bytes.fromhex('ff90'):
            end=mp+int.from_bytes(ms[mp+6:mp+10],'big');p=mp+12
            while ms[p:p+2]!=bytes.fromhex('ff93'):p+=int.from_bytes(ms[p+2:p+4],'big')+2
            body=ms[p+2:end];out.extend(PARTS.make_part(int.from_bytes(ms[mp+4:mp+6],'big'),ms[mp+10],ms[mp+11],body));mp=end
        assert ms[mp:]==bytes.fromhex('ffd9');out.extend(ms[mp:])
        wrapped=PARTS.wrap(rc,PARTS.stream_offset(rc),out)
        add(rgb,wrapped,'multiple_quality_layers','valid_unsupported','Actually encoded quality_layers=[10,2], LRCP two layers, PLT removed.')
        changed=bytearray(cod);changed[9]=2
        add(rgb,replace_main_marker(rc,'ff52',bytes(changed)),'reserved_transform2','malformed','COD transformation identifier2 is reserved in Part1.')
        add(rgb,replace_main_marker(qcc,'ff5c',None),'missing_QCD_allQCC','malformed','Mandatory main-header QCD absent even though every component has QCC.')
        add(rgb,insert_main_marker(rc,'ff5e',bytes([0,0,0])),'ROI_maxshift0','valid_unsupported','Legal RGN component0,style0,shift0 no-op.')
        changed=bytearray(cod);changed[0]=2
        add(rgb,replace_main_marker(rc,'ff52',bytes(changed)),'SOP_may_be_used','valid_unsupported','Scod declares optional SOP marker use; packet payloads unchanged.')
        codseg=bytes.fromhex('ff52')+(len(cod)+2).to_bytes(2,'big')+cod
        off=PARTS.stream_offset(rc);st=rc[off:];first=UNKNOWN.walk_sot(rc)[0]['position']-off;psot=int.from_bytes(st[first+6:first+10],'big')
        changed=bytearray(st[:first+12]+codseg+st[first+12:]);changed[first+6:first+10]=(psot+len(codseg)).to_bytes(4,'big')
        add(rgb,PARTS.wrap(rc,off,changed),'identical_tile_COD','valid_unsupported','Legal identical tile COD override, Psot includes inserted segment.')
        add(rgb,replace_main_marker(qcc,'ff5c',bytes([(rq[0]&0xe0)|3])+rq[1:]),'reserved_QCD_allQCC','malformed','Reserved main QCD style cannot be hidden by complete valid QCC overrides.')
        add(rgb,insert_main_marker(rc,'ff5d',bytes([0])+rq[:-1]),'QCC_short','malformed','Incomplete scalar-expounded quantization word.')
        add(rgb,insert_main_marker(rc,'ff5d',bytes([0,(rq[0]&0xe0)|3])+rq[1:]),'QCC_reserved_style3','malformed','Reserved QCC quantization style.')
        for base,data,reason,classification,note in cases:
            name=f'jp2_irreversible_reject_{reason}.{ext}'
            (ROOT/name).write_bytes(data)
            request=dict(encoded_path=str(ROOT/name),oracle_path=str(ROOT/base['oracle_file']))
            req,res=work/(reason+'_'+ext+'.oracle-request.json'),work/(reason+'_'+ext+'.oracle-result.json');req.write_text(json.dumps(request))
            process=subprocess.run([sys.executable,str(Path(__file__).resolve()),'--negative-oracle',str(req),str(res)],cwd=work,capture_output=True)
            status=json.loads(res.read_text()) if process.returncode==0 else dict(status='oracle-process-failure',exit_code=process.returncode,stderr=process.stderr.decode('utf8','replace'))
            if classification=='valid_unsupported':assert status['status']=='decoded',(reason,status)
            row=dict(encoded_file=name,encoded_sha256=PARTS.sha(data),classification=classification,reason=reason,note=note,
                     base_encoded_file=base['encoded_file'],oracle_status=status)
            manifest['rejected'].append(row);rejection_proofs[name]=row
    assert len(manifest['rejected'])==36
    proofs['rejected']=rejection_proofs
    proofs['normative_unconfirmed_exclusions']=[dict(profile='9/7 with no quantization style0',fixed_negative_included=False,reason='Normative combination validity not established; not classified from oracle recovery alone.')]



def write_tsv(manifest):
    header=f"Pillow {VERSIONS['pillow']} OpenJPEG {VERSIONS['openjpeg']}"
    positive=[header]
    for r in manifest['cases']:
        fields=[r['encoded_sha256'],r['encoded_file'],r['source_sha256'],r['source_file'],r['width'],r['height'],*r['block'],r['levels'],r['components'],r['mct'],*r['tile'],r['tile_count'],*r['image_origin'],*r['tile_origin'],r['max_parts_per_tile'],r['peak_join_bytes'],r['oracle_sha256'],r['oracle_file'],1]
        assert len(fields)==23
        positive.append(' '.join(map(str,fields)))
    negative=[header]+[' '.join([r['encoded_sha256'],r['encoded_file'],r['classification'],r['reason']]) for r in manifest['rejected']]
    (ROOT/'jpeg2000-irreversible-manifest.txt').write_text('\n'.join(positive)+'\n')
    (ROOT/'jpeg2000-irreversible-unsupported-manifest.txt').write_text('\n'.join(negative)+'\n')


if __name__=='__main__':
    if len(sys.argv)>1 and sys.argv[1]=='--negative-oracle':
        resource.setrlimit(resource.RLIMIT_CORE,(0,0))
        result=negative_oracle(json.loads(Path(sys.argv[2]).read_text()))
        Path(sys.argv[3]).write_text(json.dumps(result))
    elif len(sys.argv)>1 and sys.argv[1]=='--multilayer':
        resource.setrlimit(resource.RLIMIT_CORE,(0,0))
        request=json.loads(Path(sys.argv[2]).read_text());params=request['encoder_params']
        for key in ['offset','tile_offset','tile_size','codeblock_size']:params[key]=tuple(params[key])
        params['quality_layers']=[10,2]
        output=BytesIO();Image.frombytes('RGB',tuple(request['size']),Path(request['source_path']).read_bytes()).save(output,format='JPEG2000',**params)
        Path(sys.argv[3]).write_bytes(output.getvalue())
    elif len(sys.argv)>1 and sys.argv[1]=='--worker':
        resource.setrlimit(resource.RLIMIT_CORE,(0,0))
        result=encode_reference(json.loads(Path(sys.argv[2]).read_text()))
        Path(sys.argv[3]).write_text(json.dumps(result))
    else:
        work=Path(sys.argv[1]) if len(sys.argv)>1 else ROOT.parents[4]/'.test-jpeg2000-irreversible-fixtures'
        generate(work)
