use std::fs;
use std::path::PathBuf;
use std::process::exit;

use inkport_translate::codegen_seal::translate_seal;

fn usage() -> ! {
    eprintln!(
        "inkport-translate {}\n\nUSAGE:\n  inkport-translate <file.sol> --target seal --out <dir>\n\nWrites <dir>/src/lib.rs, <dir>/Cargo.toml, <dir>/.cargo/config.toml, <dir>/metadata.json",
        env!("CARGO_PKG_VERSION")
    );
    exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }

    let mut file: Option<String> = None;
    let mut target = "seal".to_string();
    let mut out: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--target" => {
                i += 1;
                target = args.get(i).cloned().unwrap_or_else(|| usage());
            }
            "--out" => {
                i += 1;
                out = Some(args.get(i).cloned().unwrap_or_else(|| usage()));
            }
            "-h" | "--help" => usage(),
            s if !s.starts_with('-') && file.is_none() => file = Some(s.to_string()),
            other => {
                eprintln!("unknown argument: {other}");
                usage();
            }
        }
        i += 1;
    }

    let file = file.unwrap_or_else(|| usage());
    let out = out.unwrap_or_else(|| usage());

    if target != "seal" {
        eprintln!("error: only --target seal is supported by this binary");
        exit(1);
    }

    let src = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {file}: {e}");
            exit(1);
        }
    };

    let art = match translate_seal(&src) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: translation failed: {e}");
            exit(1);
        }
    };

    let out_dir = PathBuf::from(&out);
    let src_dir = out_dir.join("src");
    let cargo_dir = out_dir.join(".cargo");
    if let Err(e) = fs::create_dir_all(&src_dir).and(fs::create_dir_all(&cargo_dir)) {
        eprintln!("error: cannot create output dirs: {e}");
        exit(1);
    }

    let writes = [
        (src_dir.join("lib.rs"), &art.lib_rs),
        (out_dir.join("Cargo.toml"), &art.cargo_toml),
        (cargo_dir.join("config.toml"), &art.cargo_config_toml),
        (out_dir.join("metadata.json"), &art.metadata_json),
    ];
    for (path, content) in writes {
        if let Err(e) = fs::write(&path, content) {
            eprintln!("error: cannot write {}: {e}", path.display());
            exit(1);
        }
    }

    println!(
        "wrote seal0 contract `{}` to {} (lib.rs, Cargo.toml, .cargo/config.toml, metadata.json)",
        art.crate_name,
        out_dir.display()
    );
}
