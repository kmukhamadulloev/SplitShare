fn main() {
    println!("cargo:rerun-if-changed=../../web/dist");
    assert!(
        std::path::Path::new("../../web/dist/index.html").is_file(),
        "Build the frontend first: npm ci --prefix web && npm run build --prefix web"
    );
}
