# App with SDK Libraries

This sample app imports the AFML, Rust, and Java libraries published above (afml_hello, rust_hello, java_hello) to illustrate how exports are discovered.

Steps:

1. Publish each SDK package to the local registry (`apexrc publish` inside each example library).
2. Run `apexrc add afml_hello rust_hello java_hello` inside this app.
3. `apexrc install` will copy the packages into `target/vendor/afml`, read `.afml/exports.json`, and update `target/vendor/.index.json`.
4. `apexrc build` will compile the app and `apexrc run` will invoke `afml_hello.hello` (Rust/Java exports are registered as stubs for discoverability).

