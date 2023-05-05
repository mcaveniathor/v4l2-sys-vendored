extern crate cc;
extern crate anyhow;
#[macro_use] use anyhow::Result;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("v4l-utils")
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub struct Build {
    out_dir: Option<PathBuf>,
    target: Option<String>,
    host: Option<String>,
}

pub struct Artifacts {
    include_dir: PathBuf,
    lib_dir: PathBuf,
    bin_dir: PathBuf,
    libs: Vec<String>,
    target: String,
}

fn extfiles(dir: impl AsRef<Path>, dirs: bool, extension: &str) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    if !(dir.exists() && dir.is_dir()) { anyhow::bail!("{} is not a directory", dir.display()); }
    let mut list = Vec::new();
    for entry in dir.read_dir()? {
        let entry = entry?.path();

        if entry.is_dir() {
            list.append(&mut extfiles(entry, dirs, extension.clone())?);
        }
        else {
            if let Some(ext) = entry.extension() {
                if ext == extension {
                    if dirs {
                        list.push(dir.canonicalize()?);
                    }
                    else {
                        list.push(entry.canonicalize()?);
                    }
                }
            }
        }
    }
    Ok(list)
}



impl Build {
    pub fn new() -> Build {
        Build {
            out_dir: env::var_os("OUT_DIR").map(|s| PathBuf::from(s).join("v4l2-build")),
            target: env::var("TARGET").ok(),
            host: env::var("HOST").ok(),
        }
    }

    pub fn out_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Build {
        self.out_dir = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn target(&mut self, target: &str) -> &mut Build {
        self.target = Some(target.to_string());
        self
    }

    pub fn host(&mut self, host: &str) -> &mut Build {
        self.host = Some(host.to_string());
        self
    }

    fn cmd_make(&self) -> Command {
        let host = &self.host.as_ref().expect("HOST dir not set")[..];
        if host.contains("dragonfly")
            || host.contains("freebsd")
            || host.contains("openbsd")
            || host.contains("solaris")
            || host.contains("illumos")
        {
            Command::new("gmake")
        } else {
            Command::new("make")
        }
    }


    pub fn build(&mut self) -> Artifacts {
        let target = &self.target.as_ref().expect("TARGET dir not set")[..];
        let host = &self.host.as_ref().expect("HOST dir not set")[..];
        let out_dir = self.out_dir.as_ref().expect("OUT_DIR not set");
        let build_dir = out_dir.join("build");
        let install_dir = out_dir.join("install");
        let libs = vec!["v4l2".into()];
        if build_dir.exists() {
            fs::remove_dir_all(&build_dir).unwrap();
        }
        if install_dir.exists() {
            fs::remove_dir_all(&install_dir).unwrap();
        }

        let inner_dir = build_dir.join("src");
        fs::create_dir_all(&inner_dir).unwrap();
        cp_r(&source_dir(), &inner_dir);

        let mut cc = cc::Build::new();
        cc.target(target).host(host).warnings(false).opt_level(2);


        //let mut source = extfiles(&inner_dir.join("lib/libv4l2"), false, "c").unwrap();
        let mut source = Vec::new();
        //source.push(inner_dir.join("lib/libv4l2/v4l2-plugin.c"));
        source.push(inner_dir.join("lib/libv4l2/libv4l2.c"));
        source.push(inner_dir.join("lib/libv4l2/log.c"));
        let mut inc = extfiles(&inner_dir.join("lib/include"), true, "h").unwrap();
        //inc.append(&mut extfiles(&inner_dir.join("include"), true, "h").unwrap());
        inc.push("src".into());
 
        #[cfg(target_os = "android")]
        {
            source.push(&inner_dir.join("lib/libv4l2/v4l2-plugin-android.c"));
        }

       cc
            .files(source)
            .define("PROMOTED_MODE_T", "mode_t")
            .includes(inc)
            .compile("v4l2");
        fs::remove_dir_all(&inner_dir).unwrap();

        Artifacts {
            lib_dir: install_dir.join("lib"),
            bin_dir: install_dir.join("bin"),
            include_dir: install_dir.join("include"),
            libs: libs,
            target: target.to_string(),
        }
    }

    fn run_command(&self, mut command: Command, desc: &str) {
        println!("running {:?}", command);
        let status = command.status();

        let (status_or_failed, error) = match status {
            Ok(status) if status.success() => return,
            Ok(status) => ("Exit status", format!("{}", status)),
            Err(failed) => ("Failed to execute", format!("{}", failed)),
        };
        panic!(
            "
Error {}:
    Command: {:?}
    {}: {}
    ",
            desc, command, status_or_failed, error
        );
    }
}

fn main() {
    let artifacts = Build::new().build();


}



fn cp_r(src: &Path, dst: &Path) {
    for f in fs::read_dir(src).unwrap() {
        let f = f.unwrap();
        let path = f.path();
        let name = path.file_name().unwrap();

        // Skip git metadata as it's been known to cause issues (#26) and
        // otherwise shouldn't be required
        if name.to_str() == Some(".git") {
            continue;
        }

        let dst = dst.join(name);
        if f.file_type().unwrap().is_dir() {
            fs::create_dir_all(&dst).unwrap();
            cp_r(&path, &dst);
        } else {
            let _ = fs::remove_file(&dst);
            fs::copy(&path, &dst).unwrap();
        }
    }
}

fn sanitize_sh(path: &Path) -> String {
    if !cfg!(windows) {
        return path.to_str().unwrap().to_string();
    }
    let path = path.to_str().unwrap().replace("\\", "/");
    return change_drive(&path).unwrap_or(path);

    fn change_drive(s: &str) -> Option<String> {
        let mut ch = s.chars();
        let drive = ch.next().unwrap_or('C');
        if ch.next() != Some(':') {
            return None;
        }
        if ch.next() != Some('/') {
            return None;
        }
        Some(format!("/{}/{}", drive, &s[drive.len_utf8() + 2..]))
    }
}

impl Artifacts {
    pub fn include_dir(&self) -> &Path {
        &self.include_dir
    }

    pub fn lib_dir(&self) -> &Path {
        &self.lib_dir
    }

    pub fn libs(&self) -> &[String] {
        &self.libs
    }

    pub fn print_cargo_metadata(&self) {
        println!("cargo:rustc-link-search=native={}", self.lib_dir.display());
        for lib in self.libs.iter() {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
        println!("cargo:include={}", self.include_dir.display());
        println!("cargo:lib={}", self.lib_dir.display());
        if self.target.contains("msvc") {
            println!("cargo:rustc-link-lib=user32");
        } else if self.target == "wasm32-wasi" {
            println!("cargo:rustc-link-lib=wasi-emulated-signal");
            println!("cargo:rustc-link-lib=wasi-emulated-process-clocks");
            println!("cargo:rustc-link-lib=wasi-emulated-mman");
            println!("cargo:rustc-link-lib=wasi-emulated-getpid");
        }
    }
}
