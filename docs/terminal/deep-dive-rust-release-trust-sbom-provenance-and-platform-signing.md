# Deep Dive - Rust Release Trust, SBOM, Provenance, and Platform Signing

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

После artifact-topology и package-productization research стало видно, что для package мирового уровня мало просто:

- собрать binaries
- собрать `cdylib`
- собрать `.node`
- выпустить теги

Нужно ещё отдельно проектировать:

- software bill of materials
- auditability бинарников
- dependency trust policy
- artifact provenance and signing
- platform-specific signing and installer leaves
- release evidence that external teams can actually verify

🔥 Именно здесь часто происходит неприятная подмена:

- security story прячут внутрь README
- SBOM считают "enterprise optional"
- signing и notarization вплетают прямо в core build logic
- platform installers начинают диктовать product surface
- provenance путают с semver discipline

Для universal embeddable terminal package это слишком слабый уровень.

## Primary Sources

### Rust trust and SBOM tooling

- [`cargo-auditable` crate](https://crates.io/crates/cargo-auditable)
- [`cargo-auditable` repo](https://github.com/rust-secure-code/cargo-auditable)
- [`cargo-cyclonedx` crate](https://crates.io/crates/cargo-cyclonedx)
- [`cargo-cyclonedx` repo](https://github.com/CycloneDX/cyclonedx-rust-cargo)
- [`cargo-vet` crate](https://crates.io/crates/cargo-vet)
- [`cargo-vet` repo](https://github.com/mozilla/cargo-vet)
- [`cargo-about` crate](https://crates.io/crates/cargo-about)
- [`cargo-about` repo](https://github.com/EmbarkStudios/cargo-about)

### Release and provenance-adjacent tooling

- [`cargo-release` crate](https://crates.io/crates/cargo-release)
- [`cargo-release` repo](https://github.com/crate-ci/cargo-release)
- [`git-cliff` crate](https://crates.io/crates/git-cliff)
- [`git-cliff` repo](https://github.com/orhun/git-cliff)
- [`sigstore` crate](https://crates.io/crates/sigstore)
- [`sigstore-rs` repo](https://github.com/sigstore/sigstore-rs)
- [`cosign` repo](https://github.com/sigstore/cosign)

### Platform signing and installer leaves

- [`apple-codesign` crate](https://crates.io/crates/apple-codesign)
- [`apple-platform-rs` repo](https://github.com/indygreg/apple-platform-rs)
- [`cargo-bundle` crate](https://crates.io/crates/cargo-bundle)
- [`cargo-bundle` repo](https://github.com/burtonageo/cargo-bundle)
- [`cargo-deb` crate](https://crates.io/crates/cargo-deb)
- [`cargo-deb` repo](https://github.com/kornelski/cargo-deb)
- [`cargo-generate-rpm` crate](https://crates.io/crates/cargo-generate-rpm)
- [`cargo-generate-rpm` repo](https://github.com/cat-in-136/cargo-generate-rpm)
- [`cargo-wix` crate](https://crates.io/crates/cargo-wix)
- [`cargo-wix` repo](https://github.com/volks73/cargo-wix)
- [`cargo-appimage` crate](https://crates.io/crates/cargo-appimage)
- [`cargo-appimage` repo](https://github.com/StratusFearMe21/cargo-appimage)

## Freshness signals

- `cargo-auditable 0.7.4` - repo `rust-secure-code/cargo-auditable`, `818` stars, pushed `2026-03-18`
- `cargo-cyclonedx 0.5.9` - repo `CycloneDX/cyclonedx-rust-cargo`, `162` stars, pushed `2026-03-19`
- `cargo-vet 0.10.2` - repo `mozilla/cargo-vet`, `815` stars, pushed `2026-02-26`
- `cargo-about 0.8.4` - repo `EmbarkStudios/cargo-about`, `719` stars, pushed `2026-01-28`
- `cargo-release 1.1.2 latest` - installed line `1.1.1`, repo `crate-ci/cargo-release`, `1552` stars, pushed `2026-04-17`
- `git-cliff 2.12.0` - repo `orhun/git-cliff`, `11728` stars, pushed `2026-04-12`
- `sigstore 0.13.0` - repo `sigstore/sigstore-rs`, `229` stars, pushed `2026-04-13`
- `cosign` repo `sigstore/cosign`, `5832` stars, pushed `2026-04-16`
- `apple-codesign 0.29.0` - repo `indygreg/apple-platform-rs`, `802` stars, pushed `2026-04-17`
- `cargo-bundle 0.10.0` - repo `burtonageo/cargo-bundle`, `1331` stars, pushed `2026-04-18`
- `cargo-deb 3.6.3` - repo `kornelski/cargo-deb`, `562` stars, pushed `2026-03-11`
- `cargo-generate-rpm 0.20.0` - repo `cat-in-136/cargo-generate-rpm`, `114` stars, pushed `2025-12-06`
- `cargo-wix 0.3.9` - repo `volks73/cargo-wix`
- `cargo-appimage 2.4.0` - repo `StratusFearMe21/cargo-appimage`, `79` stars, pushed `2025-08-09`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**release trust is its own outer architecture**

Healthiest shape сейчас выглядит так:

1. core crates stay free of signing and installer assumptions
2. trust evidence is generated from the workspace, not handwritten later
3. SBOM, license inventory and binary auditability are different surfaces
4. provenance/signing should happen at artifact leaves
5. platform packaging and notarization stay outside runtime truth
6. release notes and semver discipline support trust, but do not replace it

То есть не:

- "у нас есть semver и changelog, значит trust story уже норм"

и не:

- "подпишем что-нибудь в CI и всё"

а:

- explicit trust artifacts
- explicit policy
- explicit outer signing leaves

## Top 3 directions for release-trust architecture

### 1. `Trust-evidence stack around a clean core`

`🎯 10   🛡️ 9   🧠 7`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- core runtime stays packaging-neutral
- CI generates SBOM, license inventory and auditable binaries
- dependency trust has explicit policy
- artifact signing/provenance happens at release leaves
- platform installers are optional extra surfaces

Почему это лучший путь:

- keeps security and trust real without contaminating core architecture
- gives external consumers something verifiable
- scales to multiple artifact families
- works for Electron-first and non-Electron consumers alike

### 2. `Platform-package trust first`

`🎯 7   🛡️ 7   🧠 6`
Примерно `5000-11000` строк.

Идея:

- trust is mostly expressed through OS-native bundles and package managers
- `.deb`, `.rpm`, app bundles, signed installers do most of the work
- Rust-level trust tooling stays secondary

Почему это sometimes works:

- useful for standalone app story
- familiar to end-user distribution teams

Почему это weaker for our package:

- embeddable C ABI and Node consumers get less assurance
- reusable package truth becomes too app-shaped
- platform leaves start dominating the product model

### 3. `Ad hoc release notes + checksum files + tribal signing`

`🎯 2   🛡️ 3   🧠 4`
Примерно `3000-7000` строк на старт и потом дорого чинить.

Это плохой путь.

Симптомы:

- no real SBOM discipline
- no binary auditability story
- provenance depends on CI folklore
- every artifact family drifts differently

## 1. `cargo-auditable` and `cargo-cyclonedx` solve different trust jobs

This distinction matters.

### `cargo-auditable 0.7.4`

Best role:

- embed dependency metadata into production binaries
- help scanners and incident response later
- make shipped binaries more inspectable

### `cargo-cyclonedx 0.5.9`

Best role:

- emit SBOM as separate machine-readable artifact
- support supply-chain workflows and external consumers
- provide release evidence outside the binary itself

🔥 Strong rule:

**binary auditability and SBOM generation are complementary, not duplicates**

Healthy packaging story often needs both.

## 2. `cargo-vet` is policy, not scanning

`cargo-vet 0.10.2` remains important because it does something different:

- it encodes trusted-source and reviewed-dependency policy
- it is not the same as vulnerability scanning
- it is not the same as license inventory

Why that matters for this package:

- we are explicitly aiming for external reuse
- long-lived runtime dependencies need trust policy, not just CVE checks

So the split should stay clear:

- `cargo-vet` - trust policy
- `cargo-audit` / advisories - vulnerability checks
- `cargo-about` - license inventory
- `cargo-cyclonedx` - SBOM
- `cargo-auditable` - auditable shipped binaries

## 3. `cargo-about` is a real product artifact helper, not paperwork

`cargo-about 0.8.4` looks modest, but its role is stronger than it seems.

Why:

- embedders and enterprise adopters will ask about license surfaces
- multi-artifact packages make dependency visibility harder
- trust is not only about signatures

Healthy role:

- generate machine- and human-consumable license inventory
- keep distribution evidence aligned with actual workspace state

Bad role:

- manual third-party markdown maintained by hand

## 4. `sigstore-rs` is promising, but `cosign` still looks like the stronger operational donor

This split became clearer during the pass.

### `sigstore 0.13.0` / `sigstore-rs`

Good as:

- Rust-native exploration seam
- future integration point
- verification/signing logic donor

Still weaker as:

- default operational center for all artifact signing today

### `cosign`

Good as:

- mature outer signing/provenance tool
- release-pipeline donor
- practical binary/container signing reference

🔥 Practical rule:

**sigstore logic should stay in the release edge, not in runtime crates**

For this package the healthiest shape is:

- runtime knows nothing about signing
- release pipeline produces signed attestable artifacts
- host consumers can verify separately

## 5. `cargo-release` and `git-cliff` help trust, but they are not provenance

These tools are still worth taking seriously.

### `cargo-release`

Useful for:

- controlled version bumping
- release workflow discipline
- reproducible release steps

### `git-cliff`

Useful for:

- high-quality changelogs
- conventional-commit aligned release notes
- external consumer readability

But the strong rule remains:

- changelog quality supports trust
- it does not replace signing, SBOM, or auditability

## 6. `apple-codesign` is exactly the kind of thing that should stay an outer leaf

`apple-codesign 0.29.0` is a strong proof of the right boundary.

Why:

- Apple signing/notarization is real complexity
- it belongs to artifact publication and platform distribution
- it should not become the shape of your runtime

Healthy role:

- macOS artifact signing
- notarization workflows
- app/bundle leaf support

Unhealthy role:

- making Apple-specific requirements part of core build truth

## 7. `cargo-bundle`, `cargo-deb`, `cargo-generate-rpm`, `cargo-wix`, `cargo-appimage` are package leaves, not architecture centers

These tools matter, but their authority must stay limited.

### Useful interpretation

- `cargo-bundle` for app bundles
- `cargo-deb` for Debian-family packaging
- `cargo-generate-rpm` for RPM-family packaging
- `cargo-wix` for Windows installer lane
- `cargo-appimage` for AppImage lane

### Wrong interpretation

- letting one installer format define the product shape
- tying runtime versioning to one OS package worldview
- forcing embeddable consumers to think like app bundle consumers

🔥 Strong rule:

**installer families are distribution leaves above the artifact matrix**

Not:

- public truth
- runtime contract
- host-neutral SDK surface

## 8. Reproducibility should be treated as evidence discipline, not mystical purity

For this package, "reproducible enough to trust releases" matters more than ideological perfection.

Healthy interpretation:

- deterministic release workflow where feasible
- documented artifact matrix
- machine-readable evidence attached to releases
- one place where trust artifacts are generated

This is another reason to keep trust tooling in the outer release layer:

- reproducibility concerns should not dictate runtime domain model
- but they should definitely shape CI and publication policy

## 9. Recommended trust stack for this terminal package

### Strong default

- `cargo-vet` - dependency trust policy
- `cargo-about` - license inventory
- `cargo-cyclonedx` - SBOM generation
- `cargo-auditable` - auditable shipped binaries
- `cargo-release` - disciplined crate release flow
- `git-cliff` - credible changelog generation
- `cosign` and/or `sigstore` verification tooling at release edge

### Platform leaves

- `apple-codesign` for macOS signing/notarization lane
- `cargo-bundle` for app-bundle lane
- `cargo-deb` and `cargo-generate-rpm` for Linux package lanes
- `cargo-wix` for Windows installer lane
- `cargo-appimage` only as an extra Linux delivery leaf

## 10. What should stay out of core crates

- ❌ signing keys
- ❌ notarization logic
- ❌ installer manifests as domain truth
- ❌ package-manager assumptions
- ❌ CI-specific provenance glue

Core crates should only expose:

- deterministic semantics
- versioned public contracts
- metadata that outer release tooling can consume

## Final verdict

🔥 For this terminal package, the healthiest trust story is:

- clean runtime core
- explicit trust artifacts generated from the workspace
- explicit provenance/signing at release leaves
- platform packaging kept as optional outer lanes

That gives you a package people can embed and trust, without turning the Rust core into a signing/bundling framework.

## Sources

- [cargo-auditable](https://github.com/rust-secure-code/cargo-auditable)
- [cargo-cyclonedx](https://github.com/CycloneDX/cyclonedx-rust-cargo)
- [cargo-vet](https://github.com/mozilla/cargo-vet)
- [cargo-about](https://github.com/EmbarkStudios/cargo-about)
- [cargo-release](https://github.com/crate-ci/cargo-release)
- [git-cliff](https://github.com/orhun/git-cliff)
- [sigstore-rs](https://github.com/sigstore/sigstore-rs)
- [cosign](https://github.com/sigstore/cosign)
- [apple-platform-rs](https://github.com/indygreg/apple-platform-rs)
- [cargo-bundle](https://github.com/burtonageo/cargo-bundle)
- [cargo-deb](https://github.com/kornelski/cargo-deb)
- [cargo-generate-rpm](https://github.com/cat-in-136/cargo-generate-rpm)
- [cargo-wix](https://github.com/volks73/cargo-wix)
- [cargo-appimage](https://github.com/StratusFearMe21/cargo-appimage)
