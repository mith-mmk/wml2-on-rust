"""Generate sampled JPEG stage instrumentation, without editing the input file.

Usage: instrument_jpeg_stages.py decoder.rs .test-decoder-profile.rs
Copy the output into an isolated checkout for profiling, never a release build.
Slots: entropy block, IDCT block, color/MCU, send, receive, draw/MCU.
Intervals are sampled wall time, not CPU time; parallel intervals overlap.
"""
import argparse
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
if args.source.resolve() == args.output.resolve():
    parser.error("Choose a separate output file; the source is never overwritten")
text = args.source.read_text()


def replace(old, new):
    global text
    assert text.count(old) == 1, f"Instrumentation source changed: {old!r}"
    text = text.replace(old, new)


helper = """
static PROFILE_CALLS: [std::sync::atomic::AtomicU64; 6] = [const { std::sync::atomic::AtomicU64::new(0) }; 6];
static PROFILE_NS: [std::sync::atomic::AtomicU64; 6] = [const { std::sync::atomic::AtomicU64::new(0) }; 6];
fn profile_stage<T>(slot: usize, body: impl FnOnce() -> T) -> T {
    use std::sync::atomic::Ordering::Relaxed;
    let count = PROFILE_CALLS[slot].fetch_add(1, Relaxed);
    if count % 64 == 0 {
        let start = std::time::Instant::now();
        let result = body();
        PROFILE_NS[slot].fetch_add(start.elapsed().as_nanos() as u64, Relaxed);
        result
    } else { body() }
}
"""
replace("pub(crate) struct BitReader", helper + "\npub(crate) struct BitReader")
replace("units.push(idct(&coefficients));", "units.push(profile_stage(1, || idct(&coefficients)));")
replace("convert_rgb(plane, units, components, color_space.to_string(), sampling)",
        "profile_stage(2, || convert_rgb(plane, units, components, color_space.to_string(), sampling))")
replace("let ret = baseline_read(\n", "let ret = profile_stage(0, || baseline_read(\n")
replace("                            preds[slot.component_index],\n                        );",
        "                            preds[slot.component_index],\n                        ));")
replace("sender\n                            .send((std::mem::take(&mut blocks), mcu_x, mcu_y))",
        "profile_stage(3, || sender.send((std::mem::take(&mut blocks), mcu_x, mcu_y)))")
replace("receiver\n                                .recv()", "profile_stage(4, || receiver.recv())")
replace("receiver\n                    .recv()", "profile_stage(4, || receiver.recv())")
assert text.count("option.drawer.draw(x * dx, y * dy, dx, dy, &data, None)?;") == 2
text = text.replace("option.drawer.draw(x * dx, y * dy, dx, dy, &data, None)?;",
                    "profile_stage(5, || option.drawer.draw(x * dx, y * dy, dx, dy, &data, None))?;")
replace("option\n                            .drawer\n                            .draw(mcu_x * dx, mcu_y * dy, dx, dy, &data, None)?;",
        "profile_stage(5, || option.drawer.draw(mcu_x * dx, mcu_y * dy, dx, dy, &data, None))?;")
replace("        result\n    })\n}", """        eprintln!("PROFILE {:?} {:?}", PROFILE_CALLS.each_ref().map(|a|a.load(std::sync::atomic::Ordering::Relaxed)), PROFILE_NS.each_ref().map(|a|a.load(std::sync::atomic::Ordering::Relaxed)));
        result
    })
}""")
args.output.write_text(text)
