use std::path::Path;

fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/dist");

    // Rebuild the server whenever the embedded frontend changes.
    println!("cargo:rerun-if-changed={}", dist.display());

    if !dist.join("index.html").exists() {
        panic!(
            "Frontend build not found at {}.\n\
             Run `pnpm --prefix frontend build` before `cargo build`.",
            dist.display()
        );
    }
}