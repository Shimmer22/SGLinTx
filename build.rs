fn main() {
    build_data::set_GIT_BRANCH();
    build_data::set_GIT_COMMIT();
    build_data::set_GIT_DIRTY();
    build_data::set_SOURCE_TIMESTAMP();
    build_data::no_debug_rebuilds();

    configure_vo_backend();
}

fn configure_vo_backend() {
    use std::{env, path::PathBuf};

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let lvgl_enabled = env::var_os("CARGO_FEATURE_LVGL_UI").is_some();
    if target_os != "linux" || target_arch != "riscv64" || !lvgl_enabled {
        return;
    }

    let sdk_root = env::var_os("LINTX_CVI_SDK")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../LicheeRV-Nano-Build"));
    let mw_root = sdk_root.join("middleware/v2");
    let include_dir = mw_root.join("include");
    let sample_common_dir = mw_root.join("sample/common");
    let inih_dir = mw_root.join("3rdparty/inih");
    let kernel_include_dir =
        sdk_root.join("linux_5.10/build/sg2002_licheervnano_sd/riscv/usr/include");
    let sys_include_dir = mw_root.join("modules/sys/include");
    let isp_include_dir = mw_root.join("modules/isp/include/sg200x");
    let lib_dir = mw_root.join("lib");
    let third_lib_dir = lib_dir.join("3rd");
    let pkgconfig_dir = mw_root.join("pkgconfig");

    if !include_dir.exists() || !sample_common_dir.exists() || !pkgconfig_dir.exists() {
        println!(
            "cargo:warning=VO backend disabled: SDK not found under {} (set LINTX_CVI_SDK to override)",
            sdk_root.display()
        );
        return;
    }

    println!("cargo:rerun-if-env-changed=LINTX_CVI_SDK");
    println!("cargo:rerun-if-changed=src/ui/backend/vo_shim.c");

    cc::Build::new()
        .file("src/ui/backend/vo_shim.c")
        .include(&include_dir)
        .include(&sample_common_dir)
        .include(&inih_dir)
        .include(&kernel_include_dir)
        .include(&sys_include_dir)
        .include(&isp_include_dir)
        .compile("lintx_vo_shim");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-search=native={}", third_lib_dir.display());
    for lib in [
        "sample",
        "ini",
        "misc",
        "cvi_audio",
        "cvi_vqe",
        "cvi_VoiceEngine",
        "cvi_RES1",
        "tinyalsa",
        "cvi_ssp",
        "cli",
        "sys",
        "vdec",
        "vpu",
        "venc",
        "cvi_bin",
        "cvi_bin_isp",
        "isp",
        "isp_algo",
        "ae",
        "af",
        "awb",
        "sns_full",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    for lib in ["atomic", "dl", "rt", "pthread"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}
