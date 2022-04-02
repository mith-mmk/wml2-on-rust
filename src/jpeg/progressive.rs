
#[inline] // for base line huffman
fn ac_read_progressive<B: BinaryReader>(bitread: &mut BitReader<B>,ac_decode:&HuffmanDecodeTable,zz:&mut [i32],al:u32,ah:u32,eob:&mut u32) -> Result<(),Error> {
    let mut zigzag : usize= al as usize;
    let mut run = ah -al;
    if *eob >= run  {
        *eob = *eob - run;
        return Ok(())
    } else if *eob > 0 {
        run -= *eob;
        zigzag += (*eob) as usize;
    }

    loop {
        let ac = huffman_read(bitread,&ac_decode)?;
        
        let ssss = ac & 0xf;
        let rrrr = ac >> 4;
        if ssss == 0 {  //EOBn
            if rrrr == 15 { //ZRL
                zigzag = zigzag + 16;
            } else {    // G.1.2.2
                if rrrr != 0 {
                    let e = (1 << rrrr) as u32;
                    let v = bitread.get_bits(rrrr as usize)? as u32;
                    *eob = e + v;
                } else {
                    *eob = 1;
                }
            }
            if run < *eob {
                *eob -= run;
                return Ok(())   // N/A
            }
            zigzag += *eob as usize;
            *eob = 0;
        } else {
            zigzag = zigzag + rrrr as usize;
            let v = bitread.get_bits(ssss as usize)?;
            zz[zigzag] = extend(v,ssss as usize);
        }
        if zigzag >= ah as usize {
            return Ok(())
        }
        zigzag = zigzag + 1;
    }
}

fn progressive_read<B:BinaryReader>(bitread: &mut BitReader<B>,dc_decode:&HuffmanDecodeTable,ac_decode:&HuffmanDecodeTable,pred: i32,zz:&mut Vec<i32>,al:u32,ah:u32,eob:&mut u32)-> Result<(Vec<i32>,i32),Error> {
    if al == 0 {
        zz[0] = dc_read(bitread, dc_decode, pred)?;
    }
    
    if ah > 1 {
        ac_read_progressive(bitread,ac_decode, zz, al, ah, eob)?;
    }
    return Ok((zz.to_vec(),zz[0]));
}