use std::borrow::Borrow;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use autotools::Config;

fn run_command_or_fail<P, S>(dir: &str, cmd: P, args: &[S])
where
    P: AsRef<Path>,
    S: Borrow<str> + AsRef<OsStr>,
{
    let cmd = cmd.as_ref();
    let cmd = if cmd.components().count() > 1 && cmd.is_relative() {
        // If `cmd` is a relative path (and not a bare command that should be
        // looked up in PATH), absolutize it relative to `dir`, as otherwise the
        // behavior of std::process::Command is undefined.
        // https://github.com/rust-lang/rust/issues/37868
        PathBuf::from(dir)
            .join(cmd)
            .canonicalize()
            .expect("canonicalization failed")
    } else {
        PathBuf::from(cmd)
    };
    eprintln!(
        "Running command: \"{} {}\" in dir: {}",
        cmd.display(),
        args.join(" "),
        dir
    );
    let ret = Command::new(cmd).current_dir(dir).args(args).status();
    match ret.map(|status| (status.success(), status.code())) {
        Ok((true, _)) => (),
        Ok((false, Some(c))) => panic!("Command failed with error code {}", c),
        Ok((false, None)) => panic!("Command got killed"),
        Err(e) => panic!("Command failed with error: {}", e),
    }
}

fn main() {
    eprintln!("Setting up submodules");
    if !Path::new("pjproject/LICENSE").exists() {
        eprintln!("Setting up submodules");
        run_command_or_fail("../", "git", &["submodule", "update", "--init"]);
    }

    let mut conf = Config::new("pjproject");
    conf.cflag("-fPIC")
        .cflag("-Wall")
        .insource(true)
        .cflag("-DEXCLUDE_APP")
        .cflag("-DPJ_EXCLUDE_PJSUA2");
    if cfg!(debug_assertions) {
        conf.cflag("-DPJ_DEBUG=1");
        conf.cflag("-DNDEBUG=0");
    } else {
        conf.cflag("-DNDEBUG=1");
        conf.cflag("-DPJ_DEBUG=0");
    }

    if env::var("CARGO_FEATURE_STATIC").is_ok() {
        conf.enable_static();
        conf.disable_shared();
    } else {
        conf.disable_static();
        conf.enable_shared();
    }

    if tracing::enabled!(tracing::Level::ERROR) {
        conf.cflag("-DPJ_LOG_MAX_LEVEL=1");
    } else if tracing::enabled!(tracing::Level::WARN) {
        conf.cflag("-DPJ_LOG_MAX_LEVEL=2");
    } else if tracing::enabled!(tracing::Level::INFO) {
        conf.cflag("-DPJ_LOG_MAX_LEVEL=3");
    } else if tracing::enabled!(tracing::Level::DEBUG) {
        conf.cflag("-DPJ_LOG_MAX_LEVEL=4");
    } else if tracing::enabled!(tracing::Level::TRACE) {    
        conf.cflag("-DPJ_LOG_MAX_LEVEL=6");
    } else {
        conf.cflag("-DPJ_LOG_MAX_LEVEL=1");
    }

    eprintln!("Configuring pjproject...");
    let _ = conf.configure();

    eprintln!("Building pjproject...");
    run_command_or_fail("./pjproject", "make", &[] as &[&str]);
    let dst = conf.fast_build(true).build();
    eprintln!("dst: {}", dst.display());

    let pkg_config_path = dst.join("lib").join("pkgconfig");

    #[cfg(target_os = "macos")]
    run_command_or_fail(
        pkg_config_path.to_string_lossy().as_ref(),
        "sed",
        &["-i", "", "s/-lstdc++//g", "libpjproject.pc"],
    );

    eprintln!("Linking");
    std::env::set_var("PKG_CONFIG_PATH", pkg_config_path.as_os_str());

    let library = pkg_config::Config::new()
        .statik(env::var("CARGO_FEATURE_STATIC").is_ok())
        .probe("libpjproject")
        .expect("pjproject and pjproject-devel needs to be installed");
    let mut clang_args = Vec::new();

    // get the include paths from pkg-config and create a clang argument for each
    for path in library.include_paths {
        clang_args.push("-I".to_string());
        clang_args.push(
            path.to_str()
                .expect(&format!("Couldn't convert PathBuf: {:?} to str!", path))
                .to_string(),
        );
    }

    // get the define flags from pkg-config and create a clang argument for each
    for define in library.defines {
        if let Some(value) = define.1 {
            clang_args.push(format!("-D{}={}", define.0, value));
        }
    }

    eprintln!("Generating bindgen...");
    let bindings = bindgen::Builder::default()
        .clang_args(clang_args)
        .generate_comments(false)
        .allowlist_type(r"pj.*")
        .allowlist_type(r"PJ.*")
        .allowlist_var(r"pj.*")
        .allowlist_var(r"PJ.*")
        .allowlist_function(r"pj.*")
        .allowlist_function(r"PJ.*")
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Couldn't generate bindings!");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
