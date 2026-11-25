set shell := ["bash", "-euo", "pipefail", "-c"]

default: check

bootstrap:
    cargo fetch
    cd apps/desktop && flutter pub get
    cd packages/koncet_ui && flutter pub get

codegen:
    ./scripts/codegen.sh

codegen-check:
    ./scripts/check-codegen.sh

fmt:
    cargo fmt --all
    dart format apps/desktop/lib apps/desktop/test apps/desktop/integration_test packages/koncet_ui/lib packages/koncet_ui/test

fmt-check:
    cargo fmt --all -- --check
    dart format --output=none --set-exit-if-changed apps/desktop/lib apps/desktop/test apps/desktop/integration_test packages/koncet_ui/lib packages/koncet_ui/test

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

rust-test:
    cargo test --workspace

dart-analyze:
    cd apps/desktop && flutter analyze
    cd packages/koncet_ui && flutter analyze

dart-test:
    cd apps/desktop && flutter test
    cd packages/koncet_ui && flutter test

e2e:
    rm -rf /tmp/koncet-e2e
    cd apps/desktop && XDG_DATA_HOME=/tmp/koncet-e2e flutter test integration_test/app_flow_test.dart -d linux

screenshots:
    rm -rf /tmp/koncet-screenshots
    XDG_DATA_HOME=/tmp/koncet-screenshots cargo run -q -p koncet-app --example seed_demo
    cd apps/desktop && XDG_DATA_HOME=/tmp/koncet-screenshots KONCET_CAPTURE_SCREENSHOTS=1 flutter test integration_test/screenshot_test.dart -d linux

check: fmt-check clippy rust-test dart-analyze dart-test

run:
    cd apps/desktop && flutter run -d linux

build-release:
    ./scripts/build-release.sh

package: build-release
    ./scripts/package-linux.sh

version-check:
    ./scripts/check-version.sh

verify-artifacts:
    cd dist && sha256sum -c SHA256SUMS
