//! Trusted-fixture evaluation CLI; not a production streaming entrypoint.
#[path = "../../wml2/tests/common/jpegxl_progressive.rs"]
mod contract;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    contract::cli()
}
