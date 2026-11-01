# Unreleased: JPEG XL 0.8.0 decoder

- Migrate optional pixel decoding to published jxl 0.8.0's event API while retaining published 0.7.4 solely for existing bounded preflight readers. Both remain optional and pinned; the dependency graph still contains 0.7.4.
- Preserve fixed caps, supported-profile restrictions, metadata/fallible copies, orientation and cooperative cancellation; cross-check the new header projection before publication. No wider image admission or full codec heap bound is added.
- See [migration validation and tradeoffs](../jpeg-xl-080-migration.md): photo/layer performance improves in measured cases, tiny-input performance and sample executable size regress. This is a draft change, not a new release or an assertion about the published 0.0.41 package.

- Add an opt-in owned native RGB8/16 animation API with coalesced oriented RGBA16 frames, exact raw ticks/timebase/loops, cumulative output/workspace limits and atomic cancellation/error handling. This draft is a partial #43 stage; common callbacks/Abort, EX and managed animation remain pending. See [contract and validation](../jpeg-xl-buffered-animation.md) and draft [PR #93](https://github.com/mith-mmk/wml2-on-rust/pull/93).

- Connect qualified exact-ms JXL animations to common RGBA8 callbacks, own-CMS sRGB and RGBA16 EX delivery. Preserve all coalesced canvases, loop/timing, Abort cleanup and cumulative poster/frame/scratch budgets; add `ImageBufferEx::animation()`/`loop_count()` and raw resource-limited cancellation. Fractional-ms animation remains available via the rational native API and is refused by integer-ms callbacks. See [contract](../jpeg-xl-animation-callbacks.md) and draft [PR #94](https://github.com/mith-mmk/wml2-on-rust/pull/94).
