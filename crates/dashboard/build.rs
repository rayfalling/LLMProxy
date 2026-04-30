//! Build script: ensure `web/dist/` exists at compile time so the
//! `include_dir!` macro in `static_assets.rs` always succeeds, even on a
//! fresh clone where the frontend has not been built yet.

use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist = manifest_dir
        .join("..")
        .join("..")
        .join("web")
        .join("dist");

    if !dist.exists() {
        fs::create_dir_all(&dist).expect("create web/dist placeholder");
    }

    let index = dist.join("index.html");
    if !index.exists() {
        let placeholder = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>LLMProxy</title></head>
<body style="font-family:sans-serif;padding:2rem">
<h1>LLMProxy Dashboard — frontend not built</h1>
<p>Run <code>cd web &amp;&amp; npm install &amp;&amp; npm run build</code>,
then rebuild the dashboard binary.</p>
</body></html>
"#;
        fs::write(&index, placeholder).expect("write placeholder index.html");
    }

    // Re-run build.rs only when web/dist is added/removed.
    println!("cargo:rerun-if-changed=../../web/dist");
    println!("cargo:rerun-if-changed=build.rs");
}
