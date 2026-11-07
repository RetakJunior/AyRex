use std::process::Command;

fn main() {
    // FIXME: It would be nice to run this only when tests are run.
    println!("cargo:rerun-if-changed=tests/pod.c");

    let libs = system_deps::Config::new()
        .probe()
        .expect("Cannot find libspa");
    let libspa = libs.get_by_name("libspa").unwrap();

    println!("cargo:rustc-check-cfg=cfg(spa_video_info_raw_has_flags)");
    let pipewire_version = Command::new("pkg-config")
        .args(["--modversion", "libpipewire-0.3"])
        .output()
        .expect("Cannot determine the PipeWire header version");
    assert!(pipewire_version.status.success());
    let version = String::from_utf8(pipewire_version.stdout).expect("Invalid PipeWire version");
    let mut parts = version.trim().split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    let version = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );
    if version >= (0, 3, 65) {
        println!("cargo:rustc-cfg=spa_video_info_raw_has_flags");
    }

    cc::Build::new()
        .file("tests/pod.c")
        .shared_flag(true)
        .flag("-Wno-missing-field-initializers")
        .includes(&libspa.include_paths)
        .compile("pod");
}
