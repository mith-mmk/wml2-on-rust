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
    12
}
