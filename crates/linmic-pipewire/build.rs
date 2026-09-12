fn main() {
    let lib = pkg_config::Config::new()
        .probe("libpipewire-0.3")
        .expect("Install PipeWire development headers");
    let mut b = cc::Build::new();
    b.file("src/source.c").flag("-std=c11");
    for p in lib.include_paths {
        b.include(p);
    }
    b.compile("linmic_pw");
    println!("cargo:rerun-if-changed=src/source.c");
}
