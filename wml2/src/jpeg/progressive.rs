//! Progressive JPEG scan decoding helpers.

type Error = Box<dyn std::error::Error>;
use crate::draw::*;
use crate::error::*;
use crate::jpeg::decoder::*;
use crate::jpeg::header::*;
use crate::jpeg::util::print_huffman_tables;
use crate::warning::*;
use bin_rs::reader::BinaryReader;

pub fn decode_progressive<'decode, B: BinaryReader>(
    reader: &mut B,
    header: &mut JpegHaeder,
    option: &mut DecodeOptions,
    warnings: Option<ImgWarnings>,
) -> Result<Option<ImgWarnings>, Error> {
    let width = header.width;
    let height = header.height;
    let mut huffman_scan_header = require_scan_header(header)?.clone();
    let fh = require_frame_header(header)?.clone();
    let component = require_components(&fh)?.clone();
    let plane = fh.plane;
    let mut _huffman_scan_header;

    // decode
    option.drawer.init(width, height, InitOptions::new())?;

    let (mcu_size, h_max, v_max, dx, dy) = calc_mcu(&component);
    let mut bitread = BitReader::new(reader);
    let mcu_y_max = (height + dy - 1) / dy;
    let mcu_x_max = (width + dx - 1) / dx;

    let mut mcu_blocks: Vec<Vec<Vec<i32>>> = Vec::with_capacity(mcu_y_max * mcu_x_max);
    for _ in 0..mcu_y_max * mcu_x_max {
        let mut mcu_block = Vec::with_capacity(mcu_size);
        for _ in 0..mcu_size {
            let block = (0..64).map(|_| 0).collect();
            mcu_block.push(block);
        }
        mcu_blocks.push(mcu_block);
    }

    let quantization_tables = require_quantization_tables(header)?.clone();
    let huffman_tables = &mut header.huffman_tables;
    let mut loop_count = 1;
    loop {
        let mut eobrun: usize = 0;
        let (dc_decode, ac_decode) = huffman_extend(huffman_tables);

        let (ss, se, ah, al) = (
            huffman_scan_header.ss,
            huffman_scan_header.se,
            huffman_scan_header.ah,
            huffman_scan_header.al,
        );
        if option.debug_flag > 0 {
            let mut boxstr = format!("Progressive loop{} \n", loop_count);
            for i in 0..huffman_scan_header.ns {
                boxstr += &format!(
                    "Cs{} Td {} Ta {} ",
                    huffman_scan_header.csn[i],
                    huffman_scan_header.tdcn[i],
                    huffman_scan_header.tacn[i]
                );
            }
            boxstr += &format!("Ss {} Se {} Ah {} Al {}\n", ss, se, ah, al);
            option.drawer.verbose(&boxstr, None)?;
            loop_count += 1;
        }
        let scan = calc_scan(&component, &huffman_scan_header);
        let scan_slots = build_progressive_scan_slots(
            &scan,
            &quantization_tables,
            &dc_decode,
            &ac_decode,
            ss,
            se,
        )?;
        let mut preds: Vec<i32> = (0..component.len()).map(|_| 0).collect();

        let mut mcu_interval = if header.interval > 0 {
            header.interval as isize
        } else {
            -1
        };

        if huffman_scan_header.ns > 1 {
            for mcu_y in 0..mcu_y_max {
                for mcu_x in 0..mcu_x_max {
                    let mcu_block = &mut mcu_blocks[mcu_y * mcu_x_max + mcu_x];
                    for scannumber in 0..mcu_size {
                        let slot = &scan_slots[scannumber];
                        if !slot.in_scan {
                            continue;
                        }

                        let zz = &mut mcu_block[scannumber];

                        if ss == 0 {
                            if ah == 0 {
                                let pred = preds[slot.component_index];
                                let dc = slot.dc.ok_or_else(|| {
                                    Box::new(ImgError::new_const(
                                        ImgErrorKind::IllegalData,
                                        "missing progressive DC Huffman table".to_string(),
                                    )) as Error
                                })?;
                                let val = dc_read(&mut bitread, dc, pred)?;
                                zz[0] = val << al;
                                preds[slot.component_index] = val;
                            } else if bitread.get_bit()? == 1 {
                                zz[0] |= 1 << al;
                            }
                        }
                        if se > 0 {
                            let start = if ss == 0 { 1 } else { ss };
                            let ac = slot.ac.ok_or_else(|| {
                                Box::new(ImgError::new_const(
                                    ImgErrorKind::IllegalData,
                                    "missing progressive AC Huffman table".to_string(),
                                )) as Error
                            })?;
                            if ah == 0 {
                                eobrun = progressive_ac_read(
                                    &mut bitread,
                                    ac,
                                    zz,
                                    start,
                                    se,
                                    al,
                                    eobrun,
                                )?;
                            } else {
                                eobrun = successive_approximation_read(
                                    &mut bitread,
                                    ac,
                                    zz,
                                    start,
                                    se,
                                    al,
                                    eobrun,
                                )?;
                            }
                        }
                    }
                    if header.interval > 0 {
                        mcu_interval -= 1;
                        let more_mcus = mcu_y + 1 < mcu_y_max || mcu_x + 1 < mcu_x_max;
                        if mcu_interval == 0 && more_mcus {
                            bitread.read_restart()?;
                            mcu_interval = header.interval as isize;
                            preds.fill(0);
                            eobrun = 0;
                        }
                    }
                }
            }
        } else {
            let first_component = *huffman_scan_header.csn.first().ok_or_else(|| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::IllegalData,
                    "scan header has no components".to_string(),
                )) as Error
            })?;
            let mut scanfirst = 0;
            let mut i = 0;
            for scannumber in 0..scan.len() {
                let slot = &scan_slots[scannumber];
                if slot.component_index + 1 == first_component && slot.is_first {
                    scanfirst = scannumber;
                    i = slot.component_index;
                    break;
                }
            }

            // A noninterleaved scan counts component blocks, not frame MCUs.
            let columns = (width * component[i].h + dx - 1) / dx;
            let rows = (height * component[i].v + dy - 1) / dy;
            let mut remaining_blocks = columns * rows;
            for mcu_y in 0..mcu_y_max {
                for mcu_y_fix in 0..component[i].v {
                    if mcu_y * component[i].v + mcu_y_fix >= rows {
                        break;
                    }
                    for mcu_x in 0..mcu_x_max {
                        let mcu_block = &mut mcu_blocks[mcu_y * mcu_x_max + mcu_x];
                        for mcu_x_fix in 0..component[i].h {
                            if mcu_x * component[i].h + mcu_x_fix >= columns {
                                break;
                            }
                            let scannumber = scanfirst + mcu_y_fix * component[i].h + mcu_x_fix;
                            let slot = &scan_slots[scannumber];
                            if !slot.in_scan {
                                continue;
                            }

                            let zz = &mut mcu_block[scannumber];

                            if ss == 0 {
                                if ah == 0 {
                                    let pred = preds[slot.component_index];
                                    let dc = slot.dc.ok_or_else(|| {
                                        Box::new(ImgError::new_const(
                                            ImgErrorKind::IllegalData,
                                            "missing progressive DC Huffman table".to_string(),
                                        )) as Error
                                    })?;
                                    let val = dc_read(&mut bitread, dc, pred)?;
                                    zz[0] = val << al;
                                    preds[slot.component_index] = val;
                                } else if bitread.get_bit()? == 1 {
                                    zz[0] |= 1 << al;
                                }
                            }
                            if se > 0 {
                                let start = if ss == 0 { 1 } else { ss };
                                let ac = slot.ac.ok_or_else(|| {
                                    Box::new(ImgError::new_const(
                                        ImgErrorKind::IllegalData,
                                        "missing progressive AC Huffman table".to_string(),
                                    )) as Error
                                })?;
                                if ah == 0 {
                                    eobrun = progressive_ac_read(
                                        &mut bitread,
                                        ac,
                                        zz,
                                        start,
                                        se,
                                        al,
                                        eobrun,
                                    )?;
                                } else {
                                    eobrun = successive_approximation_read(
                                        &mut bitread,
                                        ac,
                                        zz,
                                        start,
                                        se,
                                        al,
                                        eobrun,
                                    )?;
                                }
                            }
                            if header.interval > 0 {
                                remaining_blocks -= 1;
                                mcu_interval -= 1;
                                if mcu_interval == 0 && remaining_blocks > 0 {
                                    bitread.read_restart()?;
                                    mcu_interval = header.interval as isize;
                                    preds.fill(0);
                                    eobrun = 0;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Progressive draws
        /*
        for mcu_y in 0..mcu_y_max {
            for mcu_x in 0..mcu_x_max {
                let mut mcu_units :Vec<Vec<u8>> = Vec::new();
                for scannumber in 0..mcu_size {
                    let (_,_,_,tq,_) = scan[scannumber];
                    let zz = &mut mcu_block[scannumber];
                    let q = quantization_tables[tq].q.clone();
                    let sq = &super::util::ZIG_ZAG_SEQUENCE;
                    let zz :Vec<i32> = (0..64).map(|i| zz[sq[i]] * q[sq[i]] as i32).collect();
                    let ff = idct(&zz);
                    mcu_units.push(ff);
                }
                let data = convert_rgb(plane,&mcu_units,&component,header.adobe_color_transform,(h_max,v_max));
                option.drawer.draw(mcu_x*dx,mcu_y*dy,dx,dy,&data,None)?;
            }
        }
        */

        loop {
            let b = bitread.next_marker();
            match b {
                Ok(marker) => {
                    match marker {
                        0xd9 => {
                            // EOI
                            for mcu_y in 0..mcu_y_max {
                                for mcu_x in 0..mcu_x_max {
                                    let mcu_block = &mut mcu_blocks[mcu_y * mcu_x_max + mcu_x];
                                    let mut mcu_units: Vec<Vec<u8>> = Vec::new();
                                    for scannumber in 0..mcu_size {
                                        let slot = &scan_slots[scannumber];
                                        let zz = &mut mcu_block[scannumber];
                                        let q = quantization_tables[slot.quant_index].q.clone();
                                        let sq = &super::util::ZIG_ZAG_SEQUENCE;
                                        let zz: Vec<i32> =
                                            (0..64).map(|i| zz[sq[i]] * q[sq[i]] as i32).collect();
                                        let ff = idct(&zz);
                                        mcu_units.push(ff);
                                    }
                                    let data = convert_rgb(
                                        plane,
                                        &mcu_units,
                                        &component,
                                        fh.color_space.to_string(),
                                        (h_max, v_max),
                                    );
                                    option.drawer.draw(
                                        mcu_x * dx,
                                        mcu_y * dy,
                                        dx,
                                        dy,
                                        &data,
                                        None,
                                    )?;
                                }
                            }
                            option.drawer.terminate(None)?;
                            return Ok(warnings);
                        }
                        0xc4 => {
                            // DHT
                            JpegHaeder::dht_read(bitread.reader, huffman_tables)?;

                            if option.debug_flag & 0x04 > 0 {
                                let str = print_huffman_tables(huffman_tables);
                                option.drawer.verbose(&str, None)?;
                            }
                        }
                        0xda => {
                            // SOS
                            _huffman_scan_header = JpegHaeder::sos_reader(bitread.reader)?;
                            huffman_scan_header = _huffman_scan_header.clone();
                            bitread.reset();
                            break;
                        }
                        0xdd => {
                            let length = bitread.reader.read_u16_be()?;
                            if length != 4 {
                                return Err(Box::new(ImgError::new_const(
                                    ImgErrorKind::IllegalData,
                                    "Invalid progressive DRI length".to_string(),
                                )));
                            }
                            header.interval = bitread.reader.read_u16_be()? as usize;
                        }
                        /*
                        0xdb => {
                            JpegHaeder::dqt_reader(bitread.reader,&mut quantization_tables)?;
                        },*/
                        0xff => { // padding
                            // offset = offset + 1;
                        }
                        0x00 => { //data
                            // skip
                        }
                        0xd0..=0xd7 => { // REST0-7
                            // skip
                        }
                        _ => {
                            let length = bitread.reader.read_u16_be()? as usize;
                            bitread.reader.skip_ptr(length - 2)?;
                        }
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}

fn progressive_ac_read<B: BinaryReader>(
    bitread: &mut BitReader<B>,
    ac_decode: &HuffmanDecodeTable,
    zz: &mut [i32],
    ss: usize,
    se: usize,
    al: usize,
    eob: usize,
) -> Result<usize, Error> {
    if eob > 0 {
        return Ok(eob - 1);
    }

    let mut zigzag = ss;
    loop {
        // F2.2.2
        let ac = huffman_read(bitread, ac_decode)?;
        let ssss = ac & 0xf;
        let rrrr = ac >> 4;
        if ssss == 0 {
            if ac == 0x00 {
                //EOB
                return Ok(0);
            }
            if rrrr == 15 {
                //ZRL
                zigzag += 16;
                continue;
            }

            let mut eob = 1_usize << rrrr;
            eob += bitread.get_bits(rrrr as usize)? as usize;
            return Ok(eob - 1);
        } else {
            zigzag += rrrr as usize;
            if zigzag <= se {
                let v = bitread.get_bits(ssss as usize)?;
                let z = extend(v, ssss as usize);
                zz[zigzag] = z << al;
            }
        }
        if zigzag >= se {
            return Ok(0);
        }
        zigzag += 1;
    }
}

// in debug
fn successive_approximation_read<B: BinaryReader>(
    bitread: &mut BitReader<B>,
    ac_decode: &HuffmanDecodeTable,
    zz: &mut [i32],
    ss: usize,
    se: usize,
    al: usize,
    mut eob: usize,
) -> Result<usize, Error> {
    let mut zigzag = ss;
    let val = 1 << al;

    if eob == 0 {
        while zigzag <= se {
            let ac = huffman_read(bitread, ac_decode)?;
            let ssss = ac & 0xf;
            let mut rrrr = ac >> 4;
            let mut bits = 0;
            if ssss == 0 {
                //EOBn
                if rrrr == 0 {
                    eob = 1;
                    break;
                } else if rrrr == 15 { //ZRL
                    // Nil
                } else {
                    // G.1.2.2
                    let e = 1 << rrrr as usize;
                    let v = bitread.get_bits(rrrr as usize)? as usize;
                    eob = e + v;
                    break;
                }
            } else if ssss == 1 {
                bits = if bitread.get_bit()? == 1 {
                    val // positive value
                } else {
                    -val // negative value
                };
            } else {
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::IllegalData,
                    "illegal data in successive approximation".to_string(),
                )));
            }

            while zigzag <= se {
                if zz[zigzag] == 0 {
                    if rrrr == 0 {
                        break;
                    }
                    rrrr -= 1;
                } else if bitread.get_bit()? == 1 {
                    if zz[zigzag] > 0 {
                        zz[zigzag] += val;
                    } else {
                        zz[zigzag] -= val;
                    }
                }
                zigzag += 1;
            }
            if zigzag <= se && bits != 0 {
                zz[zigzag] = bits;
            }
            zigzag += 1;
        }
    }

    if eob > 0 {
        while zigzag <= se {
            if zz[zigzag] != 0 && bitread.get_bit()? == 1 {
                if zz[zigzag] > 0 && zz[zigzag] & val == 0 {
                    zz[zigzag] += val;
                } else {
                    zz[zigzag] -= val;
                }
            }
            zigzag += 1;
        }
        Ok(eob - 1)
    } else {
        Ok(0)
    }
}
