# JPEG XL dependency integration recommendation

Read-only design review, 2026-11-06, against draft #96
`b5e216d991b1293dd20640422da6d184ee8d5c49` and released main0.0.41
`f2151b2408e9d04f1bc7b4e9e29679e078a57209`. This records a recommendation;
it does not select a new distributor, change dependencies, authorize an upstream
submission or claim a codec-wide heap bound.

## Recommended sequence

Keep the published0.8 pixel decoder, published0.7.4 preflight bridge and existing
fixed caps while the draft stack is reviewed. Prefer a supported upstream public
budget/admission interface in a published version as the long-term integration
route. Keep the local, hash-pinned [#96 evaluation](jpeg-xl080-entropy-budget-evaluation.md)
as reproducible API evidence until that route is agreed. The experimental facade
exposing private entropy types is not the proposed stable consumer API.

If a product deadline requires independent maintenance, a separately named,
versioned registry package is the practical alternative for a published WML2
library. That is a policy decision about ownership, updates and publishing, not
an operation authorized by the0.8 upgrade. No fork/repository/package is created
or published here. A fork alone does not complete heap tracking or justify
raising the cap.

## What the inspected public API provides

The exact0.8 archive and source hashes are pinned in the #96 snapshot. Its
`JxlDecoderOptions` provides sample limits, Main5/Main10, scan-only frame events,
orientation, alpha mode and related decode options, but no shared byte-budget or
fallible allocator hook. `bit_reader`, `entropy_coding`, `headers`, `frame`, `icc`
and `util` are private at the crate root. Public scan events do not by themselves
provide the pre-allocation ICC/TOC/layer admission currently performed by the
0.7.4 bridge. A sample count is not a limit on all live byte allocations.

These are findings for the pinned version, not a claim that future upstream
versions cannot expose them. Primary sources: the pinned
[options](https://github.com/libjxl/jxl-rs/blob/9a08ca643fd2d01f803dd0be6a5bebc9cc11e597/jxl/src/api/options.rs)
and [crate module boundary](https://github.com/libjxl/jxl-rs/blob/9a08ca643fd2d01f803dd0be6a5bebc9cc11e597/jxl/src/lib.rs).

## Distribution alternatives

| Option | Consumer behavior and cost | Recommendation / decision |
| --- | --- | --- |
| Published upstream API/version | Ordinary registry dependency reaches downstream consumers; upstream owns its API/release schedule. Requires implementation and publication first. | Preferred long-term route. External proposal/submission needs separate authorization. |
| Root `[patch.crates-io]` to a local or git copy | Useful for evaluation and an application-controlled workspace. A library's patch is ignored by the consuming workspace; it cannot silently enforce patched decoding downstream. | Keep isolated for tests; unsuitable as the sole published-library solution. |
| Git fork dependency at an immutable revision | Works for a git-sourced application/workspace, but registry publication does not distribute a git-only dependency. Requires maintained fork and consumer build policy. | No fork created. Consider only if git-only distribution is explicitly accepted. |
| Separately named registry fork, aliased as `jxl_codec` | Can propagate through a published WML2 package once actually published. Needs namespace/owner/versioning, retained BSD notices, upstream-update policy and feature/platform validation. | Alternative when upstream timing is unacceptable; user policy decision required. |
| Vendored/path copy | A root vendor/source-replacement config belongs to that workspace. A publishable path+version dependency resolves its registry version downstream, not an unpublished modification of the same name/version. | Does not avoid a distributor decision; do not assume it ships a modified0.8 crate. |
| Reimplement private parsers or swap the decoder family | Broad new parser/backend maintenance, compatibility and resource qualification. Existing reconstruction/encoder evaluations do not prove this contract. | Not justified by the small entropy patch; do not pursue implicitly. |

Cargo's [override reference](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)
explains root-only patch resolution. Its
[dependency reference](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations)
explains registry publication and path/git plus version behavior. These
constraints are why a successful local patch is not product dependency delivery.

## Minimum public contract to discuss

A decoder-level shared budget must reach every input root and retained owner:
pixel decode, ICC/preflight readers, nested maps, frame/reference owners, cloning,
reset/reuse and error paths. Independent default budgets or per-reader resets
would undercount simultaneous storage. It should expose finite requested-byte
admission and fallible allocation semantics without requiring WML2 to name
private entropy/parser structs. Caller-held output and adapter/CMS claims must
share a clearly defined quota or conservatively reserve disjoint portions.

Reservation lifetime must follow actual storage, including cached scratch after
reader completion. Allocation/growth must admit overlapping old/new payloads;
clone needs a new reservation when it copies storage; moving an owner transfers
its lease. Failed reservations and allocations roll back, storage dies before
release, and reset cannot make live cached memory disappear from accounting.
The fixed process-wide palette cache needs an explicit separate ownership policy;
#95's conservative25-slot reservation stays until an equivalent contract exists.

Removing the0.7.4 bridge additionally needs bounded pre-allocation inspection for
header geometry, ICC coded/expanded lengths, frame layers, signed crop/padding,
TOC counts/permutation/extent and references. Public BasicInfo or scan-only mode
is not a demonstrated substitute. No private parser copy or late post-allocation
check is proposed as a replacement.

#96 covers only context-map and reusable LZ77 payload storage. Entropy tables,
frame/modular/reference buffers, metadata/ICC/CMS, static/shared storage and
allocation overhead remain outside its ledger. All roots, failure paths and
actual WASM/32-bit behavior must be qualified before a broader resource envelope;
there is no cap-raising step in this recommendation.

## Decisions deliberately pending

The choice is whether to accept upstream publication timing or take ownership of
a separately distributed fork, and which distribution channels WML2 must retain.
Those decisions become necessary before any dependency rewrite or external
submission. Neither is needed for the current documentation/status reconciliation
or further isolated ownership evaluation. No new prompt is required merely to
record this recommendation. #37 remains open, with #63/#74/#96 evaluation evidence
separate from unmerged production resource work #72/#73/#95.
