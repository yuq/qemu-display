use std::{
    env,
    path::{Path, PathBuf},
};
use xshell::{cmd, Shell};

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("codegen") => codegen()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:
codegen
"
    )
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn codegen() -> Result<(), DynError> {
    let keycodemapdb = project_root().join("keycodemapdb");
    let keycodemap_src = project_root().join("keycodemap").join("src");
    let keymaps_csv = keycodemapdb.join("data").join("keymaps.csv");
    let keymap_gen = keycodemapdb.join("tools").join("keymap-gen");

    let keymaps = [
        "xorgevdev",
        "xorgkbd",
        "xorgxquartz",
        "xorgxwin",
        "osx",
        "win32",
        "x11",
    ];
    for km in &keymaps {
        let varname = format!("keymap_{}2qnum", km);
        let sh = Shell::new()?;
        let out = cmd!(
            sh,
            "{keymap_gen} code-map --lang rust --varname {varname} {keymaps_csv} {km} qnum"
        )
        .read()?;
        std::fs::write(keycodemap_src.join(format!("{}.rs", varname)), out)?;
    }
    Ok(())
}
