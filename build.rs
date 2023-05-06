extern crate cc;
extern crate bindgen;
extern crate anyhow;
use anyhow::*;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
    _bin_dir: PathBuf,
    libs: Vec<String>,
    target: String,
}


/// Recursively collects all subdirectories of `dir` which contain header files with the given extension
fn _include_dirs(dir: impl AsRef<Path>, extension: &str) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    if !(dir.exists() && dir.is_dir()) { bail!("{} is not a directory", dir.display())}
    let mut list = Vec::new();
    let mut added = false;
    for entry in dir.read_dir()? {
        let entry = entry?.path();
        if entry.is_dir() {
            list.append(&mut _include_dirs(entry, extension.clone())?);
        }
        else {
            if !added {
                if let Some(ext) = entry.extension() {
                    if ext == extension {
                        if !entry.display().to_string().contains("priv") {
                            list.push(dir.canonicalize()?); // add the directory's path to list
                            added = true;
                        }
                    }
                }
            }
        }
    }
    Ok(list)
}




/// Recursively collects the absolute paths to non-directory children of `dir` with the given extension
fn _extfiles(dir: impl AsRef<Path>, extension: impl AsRef<str>) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    if !(dir.exists() && dir.is_dir()) { bail!("{} is not a directory", dir.display()); }
    let mut list = Vec::new();
    for entry in dir.read_dir().map_err(|e| anyhow!("Failed to read contents of directory {}: {}", &dir.display(), e))? {
        let entry = entry?.path();
        if entry.is_dir() {
            list.append(&mut _extfiles(entry, extension.as_ref().clone())?);
        }
        else {
            if let Some(ext) = entry.extension() {
                if ext == extension.as_ref() {
                    list.push(entry.canonicalize()?); // add this file's path to list
                }
            }
        }
    }
    Ok(list)
}

// Returns the created path
fn create_bindings(headers: &[PathBuf], out_name: impl AsRef<str>) -> Result<PathBuf> {
    let mut builder = bindgen::Builder::default();
    for header in headers { 
        builder = builder.header(header.display().to_string());
    }
    let bindings = builder
        .generate()
        .map_err(|e| anyhow!("Failed to generate bindings: {}", e))?;

    let out_path = PathBuf::from(env::var("OUT_DIR")?).join(out_name.as_ref());
    bindings
        .write_to_file(&out_path)
        .map_err(|e| anyhow!("Failed to write bindings to file {:?}: {}", out_path.canonicalize(), e))?;
    Ok(out_path)
}

impl Build {
        pub fn build(&mut self) -> Result<Artifacts> {
        let target = &self.target.as_ref().expect("TARGET dir not set")[..];
        let host = &self.host.as_ref().expect("HOST dir not set")[..];
        let out_dir = self.out_dir.as_ref().expect("OUT_DIR not set");
        let build_dir = out_dir.join("build");
        let install_dir = out_dir.join("install");
        let libs = vec![
            "v4l",
            "v4lconvert",
            "v4l2",
        ].iter().map(|l| l.to_string()).collect();


        if build_dir.exists() {
            fs::remove_dir_all(&build_dir).map_err(|e| anyhow!("Error occurred while clearing build directory {}: {}", &build_dir.display(), e))?;
        }
        if install_dir.exists() {
            fs::remove_dir_all(&install_dir).map_err(|e| anyhow!("Error occurred while clearing install directory {}: {}", &install_dir.display(), e))?;
        }

        let inner_dir = build_dir.join("src");
        fs::create_dir_all(&inner_dir).map_err(|e| anyhow!("Error occurred while creating directory {}: {}", &inner_dir.display(), e))?;
        cp_r(&source_dir(), &inner_dir);

        // Common C source files and includesl';
        let mut v1_sources = Vec::new();
        //let mut inc = include_dirs(&inner_dir.join("lib/include"), "h")?;
        //inc.append(&mut include_dirs(&inner_dir.join("include"), "h")?);
        let mut inc = vec![inner_dir.join("lib/include")];
        inc.push(inner_dir.join("include"));
        let libpath = inner_dir.join("lib");
        v1_sources.push(libpath.join("libv4l1/libv4l1.c"));
        v1_sources.push(libpath.join("libv4l2/libv4l2.c"));
        v1_sources.push(libpath.join("libv4l1/log.c"));

        let mut sources = v1_sources.clone();
        /*
        let mut cc = cc::Build::new();
        cc.target(target).host(host).warnings(false).opt_level(2);
        cc
            .define("V4L2_PIX_FMT_NV12_16L16", "v4l2_fourcc('H', 'M', '1', '2')")
            //.define("HAVE_V4L_PLUGINS", None)
            .files(v1_sources)
            .includes(inc.clone())
            .define("PROMOTED_MODE_T", "mode_t")
            .compile("v4l");

            */
        let mut c_sources = Vec::new();
        //sources.push(libpath.join("libv4l2/v4l2-plugin.c"));
        c_sources.push(libpath.join("libv4lconvert/libv4lconvert.c"));
        c_sources.push(libpath.join("libv4lconvert/bayer.c"));
        c_sources.push(libpath.join("libv4lconvert/cpia1.c"));
        c_sources.push(libpath.join("libv4lconvert/crop.c"));
        c_sources.push(libpath.join("libv4lconvert/flip.c"));
        c_sources.push(libpath.join("libv4lconvert/helper.c"));
        c_sources.push(libpath.join("libv4lconvert/jidctflt.c"));
        c_sources.push(libpath.join("libv4lconvert/jl2005bcd.c"));
        c_sources.push(libpath.join("libv4lconvert/jpeg.c"));
        c_sources.push(libpath.join("libv4lconvert/jpeg_memsrcdest.c"));
        c_sources.push(libpath.join("libv4lconvert/jpgl.c"));
        c_sources.push(libpath.join("libv4lconvert/libv4lconvert.c"));
        c_sources.push(libpath.join("libv4lconvert/mr97310a.c"));
        c_sources.push(libpath.join("libv4lconvert/nv12_16l16.c"));
        c_sources.push(libpath.join("libv4lconvert/ov511-decomp.c"));
        c_sources.push(libpath.join("libv4lconvert/ov518-decomp.c"));
        c_sources.push(libpath.join("libv4lconvert/pac207.c"));
        c_sources.push(libpath.join("libv4lconvert/rgbyuv.c"));
        c_sources.push(libpath.join("libv4lconvert/se401.c"));
        c_sources.push(libpath.join("libv4lconvert/sn9c10x.c"));
        c_sources.push(libpath.join("libv4lconvert/sn9c2028-decomp.c"));
        c_sources.push(libpath.join("libv4lconvert/sn9c20x.c"));
        c_sources.push(libpath.join("libv4lconvert/spca501.c"));
        c_sources.push(libpath.join("libv4lconvert/spca561-decompress.c"));
        c_sources.push(libpath.join("libv4lconvert/sq905c.c"));
        c_sources.push(libpath.join("libv4lconvert/stv0680.c"));
        c_sources.push(libpath.join("libv4lconvert/tinyjpeg.c"));
        c_sources.push(libpath.join("libv4lconvert/processing/autogain.c"));
        c_sources.push(libpath.join("libv4lconvert/processing/gamma.c"));
        c_sources.push(libpath.join("libv4lconvert/processing/libv4lprocessing.c"));
        c_sources.push(libpath.join("libv4lconvert/processing/whitebalance.c"));
        sources.extend_from_slice(&c_sources);
        /*
        let mut cc = cc::Build::new();
        cc.target(target).host(host).warnings(false).opt_level(2);
        cc
            .define("V4L2_PIX_FMT_NV12_16L16", "v4l2_fourcc('H', 'M', '1', '2')")
            //.define("HAVE_V4L_PLUGINS", None)
            .files(c_sources)
            .includes(inc.clone())
            .define("PROMOTED_MODE_T", "mode_t")
            .compile("v4lconvert");
        */
        let mut v2_sources = sources;
        v2_sources.push(libpath.join("libv4l2/log.c"));
        v2_sources.push(libpath.join("libv4l1/v4l1compat.c"));
        v2_sources.push(libpath.join("libv4l-mplane/libv4l-mplane.c"));
        v2_sources.push(libpath.join("libv4l2rds/libv4l2rds.c"));
        let mut cc = cc::Build::new();
        cc.target(target).host(host).warnings(false).opt_level(2);
        cc
            .define("V4L2_PIX_FMT_NV12_16L16", "v4l2_fourcc('H', 'M', '1', '2')")
            //.define("HAVE_V4L_PLUGINS", None)
            .files(v2_sources.clone())
            .includes(inc.clone())
            .define("PROMOTED_MODE_T", "mode_t")
            .compile("v4l");

        let mut cc = cc::Build::new();
        cc.target(target).host(host).warnings(false).opt_level(2);
        cc
            .define("V4L2_PIX_FMT_NV12_16L16", "v4l2_fourcc('H', 'M', '1', '2')")
            //.define("HAVE_V4L_PLUGINS", None)
            .files(v2_sources.clone())
            .includes(inc.clone())
            .define("PROMOTED_MODE_T", "mode_t")
            .compile("v4lconvert");

        let mut cc = cc::Build::new();
        cc.target(target).host(host).warnings(false).opt_level(2);
        cc
            .define("V4L2_PIX_FMT_NV12_16L16", "v4l2_fourcc('H', 'M', '1', '2')")
            //.define("HAVE_V4L_PLUGINS", None)
            .files(v2_sources)
            .includes(inc)
            .define("PROMOTED_MODE_T", "mode_t")
            .compile("v4l2");








        //println!("cargo:rustc-link-lib=v4lconvert");
        //println!("cargo:rustc-link-lib=v4l1");
        //println!("cargo:rustc-link-lib=v4l2");
        //source.push("videodev2.h".into());
        //source.push(inner_dir.join("lib/libv4l1/v4l1compat.c"));
        #[cfg(target_os = "android")]
        source.push(&inner_dir.join("lib/libv4l2/v4l2-plugin-android.c"))?;
       


        let include_dir = install_dir.join("include");
        let lib_dir = install_dir.join("lib");
        fs::create_dir_all(&include_dir).map_err(|e| anyhow!("Error occurred while creating directory {}: {}", &include_dir.display(), e))?;
        let headers: Vec<PathBuf> = vec!["v4l-utils/lib/include/libv4l2.h", "v4l-utils/lib/include/libv4lconvert.h", "v4l-utils/lib/include/libv4l-plugin.h", "wrapper_v4l2.h", ].iter().map(|h| PathBuf::from(h)).collect();
        let _outfile = create_bindings(&headers, "v4l2_bindings.rs")?;
        fs::remove_dir_all(&inner_dir)?;
        
        Ok(Artifacts {
            include_dir,
            lib_dir,
            _bin_dir: install_dir.join("bin"),
            libs,
            target: target.to_string(),
        })
    }


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
}



fn main() -> Result<()> {
    let _artifacts = Build::new().build()?;
    Ok(())
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

fn _sanitize_sh(path: &Path) -> String {
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
