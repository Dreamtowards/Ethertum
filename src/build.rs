

fn main() {

    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=src/main.rs");

    
    println!("Running build.rs Test1234");
}