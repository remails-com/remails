
fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");

    // load frontend assets
    memory_serve::load_directory("frontend/dist");
}
