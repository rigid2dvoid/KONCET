fn main() {
    println!("cargo:rerun-if-env-changed=KONCET_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=KONCET_BUILD_DATE");
}
