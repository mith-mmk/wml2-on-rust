# Unreleased: JPEG XL 0.8.0 decoder

- Migrate optional pixel decoding to published jxl 0.8.0's event API while retaining published 0.7.4 solely for existing bounded preflight readers. Both remain optional and pinned; the dependency graph still contains 0.7.4.
- Preserve fixed caps, supported-profile restrictions, metadata/fallible copies, orientation and cooperative cancellation; cross-check the new header projection before publication. No wider image admission or full codec heap bound is added.
- See [migration validation and tradeoffs](../jpeg-xl-080-migration.md): photo/layer performance improves in measured cases, tiny-input performance and sample executable size regress. This is a draft change, not a new release or an assertion about the published 0.0.41 package.
