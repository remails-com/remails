fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");

    // load frontend assets
    let embed = !cfg!(debug_assertions);
    memory_serve::load_names_directories(
        vec![("frontend", "frontend/dist"), ("openapi", "src/static")],
        embed,
    );
}
