//! Actual wasm32 execution of the explicit finite-resource APIs.
use wml2::{jpegxl::*, limits::DecodeLimits};
#[unsafe(no_mangle)]
pub extern "C" fn verify_jxl_resources() -> u32 {
    let input=include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba8.jxl");
    let expected=include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba8-djxl.bin");
    let defaults=JxlResourceLimits::default();
    let unlimited=DecodeLimits::unlimited();
    let image=image_from_with_resource_limits(input,unlimited,defaults).unwrap();
    assert_eq!((image.width,image.height),(7,5));assert_eq!(image.buffer.as_deref().unwrap(),expected);
    for policy in [JxlResourceLimits {workspace_bytes:0,..defaults},JxlResourceLimits {internal_layers:0,..defaults},JxlResourceLimits {workspace_bytes:usize::MAX,..defaults}] {
        assert!(image_from_with_resource_limits(input,unlimited,policy).is_err());
    }
    let qhd=include_bytes!("../../wml2/tests/fixtures/review/jxl-resource-qhd.jxl");
    let error=image_from_with_resource_limits(qhd,unlimited,JxlResourceLimits {workspace_bytes:1<<30,..defaults}).err().unwrap();
    assert!(error.to_string().contains("padded frame pixels"));
    let icc=include_bytes!("../../wml2/tests/fixtures/review/jxl-icc-lossless.jxl");
    assert!(image_from_with_resource_limits(icc,unlimited,JxlResourceLimits {icc_coded_bytes:0,..defaults}).is_err());
    let brob=include_bytes!("../../wml2/tests/fixtures/review/jxl-brob-metadata.jxl");
    assert!(image_from_with_resource_limits(brob,unlimited,JxlResourceLimits {brob_bytes:216,..defaults}).is_ok());
    assert!(image_from_with_resource_limits(brob,unlimited,JxlResourceLimits {brob_bytes:215,..defaults}).is_err());
    let reference=include_bytes!("../../wml2/tests/fixtures/review/jxl-icc-lossless.srgb.rgba");
    let managed=image_from_srgb_with_resource_limits(icc,unlimited,defaults).unwrap().buffer.unwrap();
    assert_eq!(managed.len(),reference.len());
    for (a,b) in managed.chunks_exact(4).zip(reference.chunks_exact(4)) {
        for c in 0..3 {assert!(a[c].abs_diff(b[c])<=2);}assert_eq!(a[3],b[3]);
    }
    let cancelled=image_from_srgb_with_resource_limits_and_cancel(b"invalid",unlimited,JxlResourceLimits {workspace_bytes:usize::MAX,..defaults},&||true).err().unwrap();
    assert!(cancelled.is::<DecodeCancelled>());
    let calls=std::cell::Cell::new(0);
    image_from_srgb_with_resource_limits_and_cancel(icc,unlimited,defaults,&||{calls.set(calls.get()+1);false}).unwrap();
    assert!(calls.get()>10);
    for stop in 1..=calls.get() {
        let current=std::cell::Cell::new(0);
        assert!(image_from_srgb_with_resource_limits_and_cancel(icc,unlimited,defaults,&||{current.set(current.get()+1);current.get()==stop}).err().unwrap().is::<DecodeCancelled>());
    }
    assert!(image_from_with_resource_limits(input,DecodeLimits {expanded_bytes:1,..unlimited},defaults).is_err());
    // Container record storage and exact key schema under the actual ceiling.
    for file in [
        include_bytes!("../../wml2/tests/fixtures/review/jxl-brob-metadata.jxl").as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-brob-metadata-plain.jxl").as_slice(),
    ] {
        let image=image_from_with_resource_limits(file,unlimited,defaults).unwrap();
        assert_eq!(image.buffer.as_deref().unwrap(),expected);
    }
    fn add_box(out:&mut Vec<u8>,kind:&[u8;4],payload:&[u8]) {
        out.extend_from_slice(&((payload.len()+8)as u32).to_be_bytes());
        out.extend_from_slice(kind);out.extend_from_slice(payload);
    }
    let mut dense=b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    add_box(&mut dense,b"ftyp",b"jxl \0\0\0\0jxl ");
    let mut compressed=b"test".to_vec();
    compressed.extend_from_slice(include_bytes!("../../wml2/tests/fixtures/review/jxl-brob-empty.br"));
    for _ in 0..1024 {add_box(&mut dense,b"brob",&compressed);}
    let mut over=dense.clone();add_box(&mut over,b"brob",&compressed);add_box(&mut over,b"jxlc",input);
    add_box(&mut dense,b"jxlc",input);
    let image=image_from_with_resource_limits(&dense,unlimited,defaults).unwrap();
    assert_eq!(image.buffer.as_deref().unwrap(),expected);
    let metadata=image.metadata.unwrap();
    assert_eq!(metadata.keys().filter(|s|s.starts_with("JPEG XL Source Box ")).count(),1024);
    assert_eq!(metadata.keys().filter(|s|s.starts_with("JPEG XL Decoded Box ")).count(),1024);
    assert!(matches!(&metadata["JPEG XL Decoded Box 1024 74657374"],wml2::metadata::DataMap::Raw(v)if v.is_empty()));
    assert!(image_from_with_resource_limits(&over,unlimited,defaults).is_err());
    let animation=include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-main5-icc16-variable.jxl");
    let mut at=12;
    while &animation[at+4..at+8]!=b"jxlc"&&&animation[at+4..at+8]!=b"jxlp" {
        let n=u32::from_be_bytes(animation[at..at+4].try_into().unwrap())as usize;assert!(n>=8);at+=n;
    }
    let mut wrapped=animation[..at].to_vec();
    for _ in 0..17 {add_box(&mut wrapped,b"test",&[42;256]);}
    wrapped.extend_from_slice(&animation[at..]);
    let image=image_from_with_resource_limits(&wrapped,unlimited,defaults).unwrap();
    let gold=[include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-main5-icc16-variable.0.rgba16le").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-main5-icc16-variable.1.rgba16le").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-main5-icc16-variable.2.rgba16le").as_slice()];
    assert_eq!(image.animation.as_ref().unwrap().len(),3);
    for (frame,reference)in image.animation.as_ref().unwrap().iter().zip(gold) {
        for(a,b)in frame.buffer.iter().zip(reference.chunks_exact(2)){let v=u32::from(u16::from_le_bytes(b.try_into().unwrap()));assert!(a.abs_diff(((v*255+32767)/65535)as u8)<=1);}
    }
    let native=animation_rgba16_from_with_resource_limits_and_cancel(&wrapped,unlimited,defaults,&||false).unwrap();
    for(frame,reference)in native.frames.iter().zip(gold){for(a,b)in frame.rgba16.iter().zip(reference.chunks_exact(2)){assert!(a.abs_diff(u16::from_le_bytes(b.try_into().unwrap()))<=1);}}
    let(mut lo,mut hi)=(0,16<<20);
    while lo<hi {let mid=lo+(hi-lo)/2;if image_from_with_resource_limits(&wrapped,unlimited,JxlResourceLimits{workspace_bytes:mid,..defaults}).is_ok(){hi=mid}else{lo=mid+1}}
    assert!(image_from_with_resource_limits(&wrapped,unlimited,JxlResourceLimits{workspace_bytes:lo,..defaults}).is_ok());
    assert!(image_from_with_resource_limits(&wrapped,unlimited,JxlResourceLimits{workspace_bytes:lo-1,..defaults}).is_err());
    16
}
