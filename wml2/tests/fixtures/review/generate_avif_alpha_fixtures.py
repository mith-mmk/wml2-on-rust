from pathlib import Path
import struct
D=Path(__file__).resolve().parent;O=D
def box(k,v):return struct.pack('>I4s',len(v)+8,k)+v
def boxes(b):
 out=[];p=0
 while p<len(b):
  n,k=struct.unpack_from('>I4s',b,p);out.append([k,b[p+8:p+n]]);p+=n
 return out
def join(bs):return b''.join(box(k,v)for k,v in bs)
def first(bs,k):return next(v for t,v in bs if t==k)
def prop(bs):return boxes(first(boxes(first(bs,b'iprp')),b'ipco'))
rgb=boxes((D/'avif-icc-gradient-linear-premult1.avif').read_bytes());gray=boxes((D/'avif-icc-gray-linear.avif').read_bytes())
rmeta=boxes(first(rgb,b'meta')[4:]);gmeta=boxes(first(gray,b'meta')[4:]);gp=prop(gmeta)
def entries(meta):
 b=first(meta,b'iloc');assert b[:6]==bytes.fromhex('000000004400')
 n=int.from_bytes(b[6:8],'big');out=[]
 for i in range(n):
  v=b[8+i*14:22+i*14];ident,ref,count,at,size=struct.unpack('>HHHII',v);assert ref==0 and count==1;out.append((ident,at,size))
 return out
R=(D/'avif-icc-gradient-linear-premult1.avif').read_bytes();G=(D/'avif-icc-gray-linear.avif').read_bytes()
rpayloads=[(i,R[a:a+n])for i,a,n in entries(rmeta)];gpayload=next(G[a:a+n]for i,a,n in entries(gmeta))
# Retain primary/auxiliary item associations and alpha payload; replace primary pixels/properties with gray.
iprp=boxes(first(rmeta,b'iprp'));rp=boxes(first(iprp,b'ipco'));rp[:5]=gp
iprp=[[k,join(rp)if k==b'ipco'else v]for k,v in iprp]
meta=[[k,join(iprp)if k==b'iprp'else v]for k,v in rmeta]
payloads=[(1,gpayload),rpayloads[1]]
def save(name,meta,payloads):
 def loc(offset):
  b=bytes.fromhex('000000004400')+struct.pack('>H',len(payloads))
  for ident,p in payloads:b+=struct.pack('>HHHII',ident,0,1,offset,len(p));offset+=len(p)
  return b
 def render(offset):return box(b'meta',bytes(4)+join([[k,loc(offset)if k==b'iloc'else v]for k,v in meta]))
 ftyp=box(b'ftyp',first(rgb,b'ftyp'));m=render(0);m=render(len(ftyp)+len(m)+8)
 (O/('avif-icc-'+name)).write_bytes(ftyp+m+box(b'mdat',b''.join(p for i,p in payloads)))
save('gray-alpha-prem.avif',meta,payloads)
straight=[[k,(v[:4]+join([[t,p] for t,p in boxes(v[4:]) if t!=b'prem']))if k==b'iref'else v]for k,v in meta]
save('gray-alpha-straight.avif',straight,payloads)
# Actual two-auxiliary case: primary prem relation targets3, but decoder selects2.
m=[(k,v)for k,v in rmeta]
for j,(k,v)in enumerate(m):
 if k==b'iref':
  refs=boxes(v[4:]);refs=[[t,(struct.pack('>HHH',1,1,3)if t==b'prem'else p)]for t,p in refs];m[j]=(k,v[:4]+join(refs))
 if k==b'iinf':
  infos=boxes(v[6:]);extra=bytearray(infos[-1][1]);assert extra[0]==2;extra[4:6]=struct.pack('>H',3);infos.append([b'infe',bytes(extra)]);m[j]=(k,v[:4]+struct.pack('>H',len(infos))+join(infos))
 if k==b'iprp':
  ps=boxes(v)
  for z,(pk,pv)in enumerate(ps):
   if pk==b'ipma':
    # v0, 16-bit item IDs, one-byte associations; duplicate last alpha association.
    n=int.from_bytes(pv[4:8],'big');at=8;rows=[]
    for _ in range(n):
     row=pv[at:at+3+pv[at+2]];rows.append(row);at+=len(row)
    rows.append(struct.pack('>H',3)+rows[-1][2:]);ps[z]=[pk,pv[:4]+struct.pack('>I',len(rows))+b''.join(rows)]
  m[j]=(k,join(ps))
save('prem-other-alpha.avif',m,rpayloads+[(3,gpayload)])
print('Wrote two independent-codec-payload composite fixtures')
