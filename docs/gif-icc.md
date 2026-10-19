# GIF source ICC retention

The GIF decoder recognizes the exact 11-byte application identifier and
authentication code `ICCRGBG1012`. As specified in
[ICC.1:2004-10 Annex B.6](https://www.color.org/ICC1V42.pdf#page=84), the profile
is the application data sub-blocks joined in file order, without sequence
fields. Existing `NETSCAPE2.0`, palette, transparency and frame timing behavior
is preserved.

After a complete GIF stream, one carrier produces `Source ICC Profile` as
`DataMap::ICCProfile`. Multiple carriers instead produce independent
`GIF Source ICC Profile 1`, `GIF Source ICC Profile 2`, etc. These indices
identify carrier order, not frame ownership. Multiple carriers are never
concatenated into a fabricated profile or advertised as one global profile.
No source ICC event is emitted for an incomplete stream.

The bytes are opaque provenance, including empty, short or unsupported profile
payloads. They are not validated as transformable ICC and are never declared
as active `ICC Profile` metadata. This patch performs no automatic CMS and
makes no sRGB output claim. Explicit color-managed GIF output and per-frame
profile applicability remain separate work.

The aggregate metadata budget charges each 11-byte identifier, retained
`Vec<u8>` record, payload byte and published metadata key. Payload is charged
before fallible growth and binary reads use bin-rs. Empty repeated carriers
therefore also consume the budget. Sub-block boundaries do not reset it.
Direct calls to `gif::decoder::decode` establish a default metadata ledger;
calls within an existing guarded decode preserve its ledger and limits.
The direct legacy API's existing image lifecycle remains caller-managed.
The EX decode dispatcher and `ImageBufferEx` do not support GIF animation;
this acquisition work does not extend their supported formats.

The new source metadata callback may arrive after all frames. Its errors
propagate, and Abort stops further source delivery and terminates once.
The ordinary top-level `begin_image` boundary clears prior-image metadata
when an `ImageBuffer` is reused. No new dependencies are required.
