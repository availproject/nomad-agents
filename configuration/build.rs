//! Fetches latest configs from `CONFIG_BASE_URI` and stores them in
//! `configuration/configs`. To disable this feature pre-build or pre-test, set
//! the environment variable `BUILD_DISABLED`.

use std::{fs, io::Write, path::PathBuf};

const DEFINITIONS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/definitions.ts"));
const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/types.rs"));

const OUTPUT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/wasm/types.rs");

const CONFIG_BASE_URI: &str = "https://nomad-xyz.github.io/config";
const ENVS: &[&str] = &["development", "staging", "production"];
const CONFIG_BASE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/configs");

fn env_json(env: &str) -> String {
    format!("{}.json", env)
}

fn config_uri(env: &str) -> String {
    format!("{}/{}", CONFIG_BASE_URI, env_json(env))
}

fn config_path(env: &str) -> PathBuf {
    PathBuf::from(CONFIG_BASE_DIR).join(env_json(env))
}

async fn fetch_config(env: &str) -> eyre::Result<String> {
    let uri = config_uri(env);
    Ok(reqwest::get(uri).await?.text().await?)
}

fn store_config(env: &str, contents: &str) -> eyre::Result<()> {
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(config_path(env))
        .unwrap();

    f.write_all(contents.as_ref())?;
    Ok(())
}

async fn get_configs() -> eyre::Result<()> {
    let (first, second, third) = tokio::join!(
        fetch_config(ENVS[0]),
        fetch_config(ENVS[1]),
        fetch_config(ENVS[2]),
    );

    // We do it this way so that if fetches fail we update all possible
    // However, if disk access is erroring, we error
    if let Ok(first) = first {
        store_config(ENVS[0], &first)?
    }
    if let Ok(second) = second {
        store_config(ENVS[1], &second)?
    }
    if let Ok(third) = third {
        store_config(ENVS[2], &third)?;
    }
    Ok(())
}

fn gen_wasm_bindgen() -> eyre::Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(OUTPUT_FILE)
        .unwrap();

    writeln!(f, "//! THIS IS AUTOGENERATED CODE, DO NOT EDIT")?;
    writeln!(
        f,
        "//! Please edit `data/definitions.ts` and `data/types.rs`"
    )?;
    writeln!(f, "use wasm_bindgen::prelude::*;")?;
    writeln!(
        f,
        r###"
#[wasm_bindgen(typescript_custom_section)]
const _: &'static str = r#""###
    )?;
    f.write_all(DEFINITIONS.as_ref())?;
    writeln!(f, r###""#;"###)?;
    writeln!(f)?;
    f.write_all(TYPEDEFS.as_ref())?;

    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!(
        "cargo:rerun-if-changed={}",
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/definitions.ts")
    );
    println!(
        "cargo:rerun-if-changed={}",
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/types.rs")
    );
    println!(
        "cargo:rerun-if-changed={}",
        concat!(env!("CARGO_MANIFEST_DIR"), "/configs/production.json")
    );
    println!(
        "cargo:rerun-if-changed={}",
        concat!(env!("CARGO_MANIFEST_DIR"), "/configs/development.json")
    );
    println!(
        "cargo:rerun-if-changed={}",
        concat!(env!("CARGO_MANIFEST_DIR"), "/configs/staging.json")
    );
    gen_wasm_bindgen()?;

    // don't re-fetch configs if programmer disables build
    if std::env::var("BUILD_DISABLED").is_err() {
        get_configs().await?;
    }

    Ok(())
}
