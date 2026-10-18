#!/usr/bin/env python3
"""Independent packed-header/override reduction controls; no codec source changes."""
import argparse
import hashlib
import json
from pathlib import Path
import random
import resource
import runpy
import struct
import subprocess

ROOT=Path(__file__).resolve().parent
resource.setrlimit(resource.RLIMIT_CORE,(0,0))


def segment(marker,payload):
    return struct.pack('>HH',marker,len(payload)+2)+payload


def headers(data,start,end_marker):
    items=[]
    pos=start
    while struct.unpack_from('>H',data,pos)[0]!=end_marker:
        marker,length=struct.unpack_from('>HH',data,pos)
        items.append((pos,marker,data[pos:pos+length+2]))
        pos+=length+2
    return items,pos


def parts(data):
    main,pos=headers(data,2,0xff90)
    output=[]
    while data[pos:pos+2]==b'\xff\x90':
        size=struct.unpack_from('>I',data,pos+6)[0]
        assert size
        items,sod=headers(data,pos+12,0xff93)
        output.append(dict(start=pos,end=pos+size,sot=data[pos:pos+12],headers=items,body=data[sod+2:pos+size],body_start=sod+2))
        pos+=size
    assert data[pos:]==b'\xff\xd9'
    return main,output


def packet_pieces(body):
    positions=[]
    pos=0
    while True:
        at=body.find(b'\xff\x91\0\x04',pos)
        if at<0:break
        positions.append(at)
        pos=at+6
    assert positions and positions[0]==0
    return [body[a:b] for a,b in zip(positions,positions[1:]+[len(body)])]


def split_packet(piece):
    eph=piece.index(b'\xff\x92',6)
    return piece[:6],piece[6:eph+2],piece[eph+2:]


def length_bytes(value):
    groups=[value&127]
    value>>=7
    while value:
        groups.append((value&127)|128)
        value>>=7
    return bytes(reversed(groups))


def rebuild(main,part_records,packed='inline'):
    prefix=b'\xff\x4f'+b''.join(item for _,m,item in main if m not in [0xff55,0xff60])
    built=[]
    ppm=b''
    ppt_indices={}
    for p in part_records:
        body=p['body']
        tile_header=b''.join(item for _,m,item in p['headers'] if m!=0xff61)
        if packed!='inline':
            decoded=[split_packet(piece) for piece in packet_pieces(body)]
            packet_headers=b''.join(h for _,h,_ in decoded)
            body=b''.join(sop+payload for sop,_,payload in decoded)
            if packed=='ppt':
                tile=struct.unpack_from('>H',p['sot'],4)[0]
                ppt_index=ppt_indices.get(tile,0)
                chunks=[packet_headers[a:a+57]for a in range(0,len(packet_headers),57)]
                tile_header+=b''.join(segment(0xff61,bytes([ppt_index+i])+chunk)for i,chunk in enumerate(chunks))
                ppt_indices[tile]=ppt_index+len(chunks)
            else:
                ppm+=struct.pack('>I',len(packet_headers))+packet_headers
        sot=bytearray(p['sot'])
        total=12+len(tile_header)+2+len(body)
        struct.pack_into('>I',sot,6,total)
        built.append(bytes(sot)+tile_header+b'\xff\x93'+body)
    if packed=='ppm':
        prefix+=b''.join(segment(0xff60,bytes([i])+ppm[a:a+73]) for i,a in enumerate(range(0,len(ppm),73)))
    # Exact rebuilt tile-part lengths, with explicit two-byte tile indices.
    prefix+=segment(0xff55,b'\0\x60'+b''.join(p[4:6]+struct.pack('>I',len(p)) for p in built))
    return prefix+b''.join(built)+b'\xff\xd9'


def generate(compress,decompress,work):
    work=Path(work).resolve()
    assert work.name.startswith('.test')
    work.mkdir(exist_ok=True)
    helper=runpy.run_path(str(ROOT/'generate_jpeg2000_reduced.py'))
    codestream=helper['codestream']
    geometry=helper['geometry']
    reference=helper['packed_reference']
    declare=runpy.run_path(str(ROOT/'generate_jpeg2000_ex_color.py'))['declare_rgb']
    wrap=runpy.run_path(str(ROOT/'generate_jpeg2000_ex_gray.py'))['wrap']
    entries=[]
    details=[]

    def encode(label,bits,nc,wavelet,levels=3,tiled=False,mode=0,poc=False,layers=True,split_parts=True):
        w,h=(32,32)if tiled else(33,29)
        rng=random.Random(170321+bits+nc)
        maximum=(1<<bits)-1
        values=[rng.randrange(maximum+1) for _ in range(w*h*nc)]
        if nc==4:
            values[3*w*h:]=[0 if i%5==0 else maximum for i in range(w*h)]
        raw=work/'source.rawl'
        raw.write_bytes(bytes(values)if bits==8 else struct.pack('<'+str(len(values))+'H',*values))
        path=work/(label+'.jp2')
        command=[compress,'-i',str(raw),'-o',str(path),'-F',f'{w},{h},{nc},{bits},u','-n',str(levels+1),'-mct',str(int(nc>1)),'-SOP','-EPH','-PLT','-TLM','-b','16,16','-threads','1']
        if layers:
            command+=['-r','8,4,1']
            if split_parts and not poc:command+=['-TP','L']
        if tiled:command+=['-t','16,16']
        else:command+=['-d','3,5']
        if mode:command+=['-M',str(mode)]
        if wavelet=='97':command+=['-I']
        if poc:command+=['-POC',f'T1=0,0,3,2,{nc},LRCP/T1=2,0,3,4,{nc},CPRL']
        result=subprocess.run(command,capture_output=True,text=True)
        assert result.returncode==0,(label,result.stderr)
        data=path.read_bytes()
        if nc>1:data=declare(data,[(0,0,1),(1,0,2),(2,0,3),(3,1,0)]if nc==4 else None)
        return data,codestream(data),command

    def save(label,raw,template,bits,nc,wavelet,provenance,command,containers=True,supported=True):
        variants=['jp2']if nc==4 else['j2k','jp2']if containers else['j2k']
        hashes=[]
        for ext in variants:
            name=f'jp2_reduced_packet_{label}_{bits}b_{nc}c_{wavelet}.{ext}'
            data=raw if ext=='j2k' else wrap(raw,template)
            (ROOT/name).write_bytes(data)
            g=geometry(data)
            refs=[]
            for reduce in range(g['min_levels']+1):
                shape=((g['x1']+(1<<reduce)-1)//(1<<reduce)-(g['x0']+(1<<reduce)-1)//(1<<reduce),(g['y1']+(1<<reduce)-1)//(1<<reduce)-(g['y0']+(1<<reduce)-1)//(1<<reduce))
                process=subprocess.run([decompress,'-i',str(ROOT/name),'-o',str(work/'reference.pgx'),'-r',str(reduce),'-threads','1'],capture_output=True,text=True)
                assert process.returncode==0,(name,reduce,process.stdout,process.stderr)
                values=reference(work,bits,nc,shape)
                refs.append(hashlib.sha256(values).hexdigest())
            entry=dict(input=name,provenance=provenance,encode_command=command)
            if supported: entries.append(entry)
            details.append(dict(entry,supported_reduced_profile=supported,sha256=hashlib.sha256(data).hexdigest(),geometry=g,reference_hashes_including_zero=refs))
            hashes.append(refs)
        assert all(x==hashes[0]for x in hashes)
        return hashes[0]

    profiles=[('structural',8,1,'53',0,False),('colour',12,4,'53',0,False),('irreversible',16,3,'97',0,False),('segmented',16,1,'97',38,False),('bypass',12,3,'53',3,False),('poc',8,1,'53',0,True)]
    for label,bits,nc,wavelet,mode,poc in profiles:
        template,raw,command=encode(label,bits,nc,wavelet,mode=mode,poc=poc)
        main,records=parts(raw)
        if poc:assert any(m==0xff5f for _,m,_ in main)or any(m==0xff5f for p in records for _,m,_ in p['headers'])
        hashes=[]
        for packed in ['inline','ppt','ppm']:
            if packed=='ppt':
                _,single,_=encode(label+'_single',bits,nc,wavelet,mode=mode,poc=poc,split_parts=False)
                packed_main,packed_records=parts(single)
            else:packed_main,packed_records=main,records
            data=rebuild(packed_main,packed_records,packed)
            supported=packed!='ppt' or len(packed_records)==1
            hashes.append(save(label+'_'+packed,data,template,bits,nc,wavelet,'OpenJPEG encoded SOP/EPH/PLT/TLM multi-layer packets; independently verified '+packed+' repacking, chunked packed headers',command,supported=supported))
        assert hashes[0]==hashes[1]==hashes[2],label

    template,raw,command=encode('ppt_multi',8,1,'53')
    m,p=parts(raw)
    deferred=rebuild(m,p,'ppt')
    deferred_path=ROOT/'jp2_reduced_ppt_multi_part_deferred.j2k'
    deferred_path.write_bytes(deferred)
    for reduce in range(4):
        result=subprocess.run([decompress,'-i',str(deferred_path),'-o',str(work/'reference.pgx'),'-r',str(reduce),'-threads','1'],capture_output=True,text=True)
        assert result.returncode==0,(result.stdout,result.stderr)

    # Component override: keep an unused deeper default, but use the shallower
    # component's original code-block packets and quantization.
    for bits,wavelet in [(8,'53'),(16,'97')]:
        template,deep,command=encode('coc_deep',bits,1,wavelet,layers=False)
        _,shallow,_=encode('coc_shallow',bits,1,wavelet,levels=2,layers=False)
        main,_=parts(deep)
        shallow_main,shallow_parts=parts(shallow)
        cod=next(item[4:]for _,m,item in shallow_main if m==0xff52)
        qcd=next(item[4:]for _,m,item in shallow_main if m==0xff5c)
        main.append((0,0xff53,segment(0xff53,b'\0\0'+cod[5:])))
        main.append((0,0xff5d,segment(0xff5d,b'\0'+qcd)))
        save('coc',rebuild(main,shallow_parts),template,bits,1,wavelet,'Independent shallower component packets with COC/QCC and deeper unused main defaults',command)

        # Three independent components with distinct decomposition counts and
        # MCT disabled; no artificial full-resolution resampling oracle.
        deep_main,deep_parts=parts(deep)
        assert len(deep_parts)==len(shallow_parts)==1
        high_packets=packet_pieces(deep_parts[0]['body'])
        low_packets=packet_pieces(shallow_parts[0]['body'])
        assert len(high_packets)==4 and len(low_packets)==3
        mixed=[]
        for resolution in range(4):
            for c in range(3):
                source=low_packets if c==1 else high_packets
                if resolution>=len(source):continue
                packet=bytearray(source[resolution])
                packet[4:6]=struct.pack('>H',len(mixed))
                mixed.append(bytes(packet))
        main=[]
        for pos,m,item in deep_main:
            if m==0xff51:
                b=bytearray(item[4:]);b[34:36]=b'\0\3';b=b[:36]+b[36:39]*3;item=segment(m,b)
            main.append((pos,m,item))
        main.append((0,0xff53,segment(0xff53,b'\1\0'+cod[5:])))
        main.append((0,0xff5d,segment(0xff5d,b'\1'+qcd)))
        record=dict(deep_parts[0]);record['body']=b''.join(mixed);record['headers']=[(0,0xff58,segment(0xff58,b'\0'+b''.join(length_bytes(len(p))for p in mixed)))]
        raw=rebuild(main,[record])
        save('mixed_coc',raw,template,bits,3,wavelet,'Independent component packets with levels3/2/3, COC/QCC, MCT disabled',command,containers=False)
        if bits==8:
            bad=bytearray(raw)
            cod_position=next(pos for pos,m,_ in headers(raw,2,0xff90)[0]if m==0xff52)
            bad[cod_position+8]=1
            (ROOT/'jp2_reduced_mixed_coc_mct_unsupported.j2k').write_bytes(bad)

        template,deep,command=encode('tile_deep',bits,1,wavelet,tiled=True,layers=False)
        _,shallow,_=encode('tile_shallow',bits,1,wavelet,levels=2,tiled=True,layers=False)
        main,records=parts(deep)
        low_main,low_records=parts(shallow)
        cod=next(item for _,m,item in low_main if m==0xff52)
        qcd=next(item for _,m,item in low_main if m==0xff5c)
        records[1]=low_records[1]
        records[1]['headers']=[(0,0xff52,cod),(0,0xff5c,qcd)]+records[1]['headers']
        save('tile_levels',rebuild(main,records),template,bits,1,wavelet,'Independent tile packets with per-tile levels3/2/3/3 and actual tile COD/QCD overrides',command)
        save('tile_levels_ppt',rebuild(main,records,'ppt'),template,bits,1,wavelet,'Independent per-tile level overrides with one-part chunked PPT per tile',command)

    # Preserve a legitimate one-pixel-wide existing edge tile as a standalone
    # codestream. Full decode is valid; any positive reduction has zero width.
    original=codestream((ROOT/'jp2_reduced_empty_edge_tiles_8b_1c_53.jp2').read_bytes())
    main,records=parts(original)
    last=records[-1]
    new_main=[]
    for pos,m,item in main:
        if m==0xff51:
            b=bytearray(item[4:]);struct.pack_into('>8I',b,2,66,54,65,49,16,16,65,49);item=segment(m,b)
        new_main.append((pos,m,item))
    last=dict(last);sot=bytearray(last['sot']);sot[4:6]=b'\0\0';sot[10:12]=b'\0\1';last['sot']=bytes(sot)
    empty=rebuild(new_main,[last])
    path=ROOT/'jp2_reduced_empty_global.j2k';path.write_bytes(empty)
    result=subprocess.run([decompress,'-i',str(path),'-o',str(work/'reference.pgx'),'-threads','1'],capture_output=True,text=True)
    assert result.returncode==0,result.stderr
    assert (work/'reference_0.pgx').read_bytes().split(b'\n',1)[0].endswith(b' 1 5')

    # Offsets for deliberate late/discarded-data mutations, independent of the
    # implementation parser and identical in native and WASM tests.
    path=ROOT/'jp2_reduced_packet_structural_inline_8b_1c_53.j2k'
    raw=path.read_bytes();main,records=parts(raw);last=records[-1];pieces=packet_pieces(last['body']);piece=pieces[-1];eph=piece.index(b'\xff\x92',6);packet_at=last['end']-len(piece)
    plt=next((pos,item)for pos,m,item in reversed(last['headers'])if m==0xff58)
    tlm=next(pos for pos,m,_ in main if m==0xff55)
    fields=[path.name,packet_at+eph,packet_at+eph+2,last['end'],plt[0]+len(plt[1])-1,tlm+8+6*(len(records)-1),last['start']+6]
    assert fields[2]<fields[3]
    (ROOT/'jpeg2000-reduced-packet-positions.txt').write_text(' '.join(map(str,fields))+'\n')
    report=dict(provenance='Original OpenJPEG packets; independently decoded full/reduced after exact marker/packet repacking',deferred_multi_part_ppt=dict(input='jp2_reduced_ppt_multi_part_deferred.j2k',openjpeg_full_and_reduced_decodes=4,reason='Existing WML2 per-part header contract preserved; reduced API explicitly rejects tile-wide multi-part PPT'),files=details,empty_global_control='jp2_reduced_empty_global.j2k',unsupported_mct='jp2_reduced_mixed_coc_mct_unsupported.j2k',mutation_offsets=fields)
    (ROOT/'jpeg2000-reduced-packets.json').write_text(json.dumps(report,indent=2)+'\n')
    return entries


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compress',required=True);parser.add_argument('--decompress',required=True);parser.add_argument('--work-dir',type=Path,required=True)
    a=parser.parse_args();out=generate(a.compress,a.decompress,a.work_dir);print(json.dumps(dict(profiles=len(out))))
