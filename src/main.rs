mod commands;
mod object;
mod repository;

use anyhow::{Result, bail};
use std::env;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);

    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.as_str() {
        "init" => {
            let path = args.next().unwrap_or_else(|| ".".to_string());
            commands::init(&path)?;
        }

        "hash-object" => {
            let mut write = false;
            let mut path: Option<String> = None;
            let mut stdin_mode = false;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "-w" => write = true,
                    "--stdin" => stdin_mode = true,
                    other => path = Some(other.to_string()),
                }
            }

            commands::hash_object(path.as_deref(), stdin_mode, write)?;
        }

        "cat-file" => {
            let Some(mode) = args.next() else {
                bail!("missing cat-file mode: expected -p, -t, or -s");
            };

            let Some(hash) = args.next() else {
                bail!("missing object hash");
            };

            commands::cat_file(&mode, &hash)?;
        }

        _ => {
            bail!("unknown command: {command}");
        }
    }
    Ok(())
}

fn print_usage() {
    eprintln!(
        r#"git

Usage:
    git init [path]
    git hash-object [-w] <path>
    git hash-object [-w] --stdin
    git cat-file -p <hash>
    git cat-file -t <hash>
    git cat-file -s <hash>
"#
    );
}
