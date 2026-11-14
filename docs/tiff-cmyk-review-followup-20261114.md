# TIFF CMYK review follow-up (2026-11-14)

Independent review identified that a later page with CMYK photometric type caused
all decode errors to become fatal. This was too broad: even a corrupt unprofiled
CMYK strip stopped the file, whereas TIFF historically warns and continues after
an ordinary later-page decode failure.

The decoder now marks errors only at CMYK converter preparation and active-ICC
conversion calls. Marked CMS failures remain fatal; unrelated strip, container,
compression and no-ICC conversion errors retain best-effort warning behavior.
The internal wrapper preserves the error's text and source chain. No error-text
matching or photometric-only classification selects the fatal path.

Focused regression tests passed with color management on (two tests) and off
(one test), for Classic and BigTIFF:

- RGB first page, corrupt CMYK second page, RGB third page: warn and preserve the
  first and third pixel callbacks. Tested without ICC, with malformed ICC, and
  with valid synthetic ICC when enabled.
- RGB first page, valid ICC CMYK second page under an 8192-byte expanded budget,
  RGB third page: return the CMYK converter workspace error; only the first page
  has published pixels and the third page is not decoded.

This preserves the intentionally narrow guarantee: a fatal later-page CMS error
can follow previously delivered pages; it is not whole-document rollback. A
transform failure injected after successful inner chunks remains outside this
specific regression test. Full final-head CI is checked separately before merge.
