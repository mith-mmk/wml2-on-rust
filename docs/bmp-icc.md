# BMP V5 embedded ICC

The decoder acquires embedded ICC only from a declared 124-byte V5 DIB with
`PROFILE_EMBEDDED` (`0x4d424544`). The profile offset is relative to the DIB
header, as defined by Microsoft's
[BITMAPV5HEADER contract](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapv5header).
`PROFILE_LINKED` never opens a profile filename or URL.

The profile must lie within the actual input and any nonzero declared file
size, outside the header, palette and pixel extent. Both before-pixel and
after-pixel layouts are supported. Uncompressed pixel extents are calculated
from padded row geometry; compressed extents use `biSizeImage`. A compressed
carrier with unknown pixel size cannot put its profile after the pixel start.
Invalid extents return an error before profile allocation or callbacks.

Ordinary BMP pixels preserve `Source ICC Profile` as opaque bytes. A declared
RGB identity additionally produces `ICC Profile` for the delivered RGB
samples; short, Gray, CMYK or other identities remain source-only. Identity
inspection is not full ICC validity or transform-capability validation, and
no automatic CMS is performed. Unsupported native RGB profiles remain
retainable without compiling a transform.

For BI_PNG/BI_JPEG, the outer profile is instead `BMP Source ICC Profile`.
The embedded codec's own profile metadata is preserved, avoiding an arbitrary
choice between two different carriers. Each retained copy and key consumes
the shared metadata budget. Default direct decoder calls also establish a
ledger. Fallible allocation and bin-rs exact binary reads are used.

The new profile events occur before pixel initialization. Abort stops there
and terminates once, while callback errors propagate. The ordinary top-level
image boundary clears prior-image profiles when the same buffer is reused.
