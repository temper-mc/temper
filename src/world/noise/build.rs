use heck::ToShoutySnakeCase;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use temper_assets::asset_path;

const NOISE_PARAMETER_PATH: &str = asset_path!("data", "minecraft", "worldgen", "noise");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NoiseParameter {
    amplitudes: Vec<f64>,
    first_octave: i32,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", NOISE_PARAMETER_PATH);

    let path = Path::new(NOISE_PARAMETER_PATH);
    let mut param_map = HashMap::new();

    match traverse_directory(path, &mut param_map) {
        Ok(_) => {}
        Err(err) => {
            println!("cargo::error=Failed to gather noise parameters: {:?}", err);
            return;
        }
    }

    let mut constants = Vec::new();
    let mut match_arms = Vec::new();

    for (item, params) in param_map.into_iter() {
        let item_prefixed = format!("minecraft:{}", item);

        let ident = item.replace("/", "_");
        let ident = format_ident!("{}", ident.to_shouty_snake_case());

        let amplitudes = params.amplitudes;
        let first_octave = params.first_octave;

        constants.push(quote! {
            pub const #ident: Self = Self {
                name: #item_prefixed,
                amplitudes: &[#(#amplitudes),*],
                first_octave: #first_octave,
            };
        });

        match_arms.push(quote! {
            #item | #item_prefixed => Some(&Self::#ident),
        });
    }

    let contents = quote! {
        impl NoiseParameter {
            #(#constants)*

            pub fn get_by_name<'a>(name: impl AsRef<str>) -> Option<&'a Self> {
                match name.as_ref() {
                    #(#match_arms)*
                    _ => None,
                }
            }
        }
    };

    let out_dir_str = std::env::var("OUT_DIR").expect("no OUT_DIR found");
    let out_dir = Path::new(&out_dir_str);

    fs::write(out_dir.join("parameters_impl.rs"), contents.to_string())
        .expect("failed to write to out dir");
}

fn traverse_directory(
    path: impl AsRef<Path>,
    param_map: &mut HashMap<String, NoiseParameter>,
) -> Result<(), (std::io::Error, String)> {
    let path = path.as_ref();

    for entry in path
        .read_dir()
        .map_err(|e| (e, path.to_string_lossy().to_string()))?
    {
        let entry = entry.map_err(|e| (e, path.to_string_lossy().to_string()))?;

        if entry
            .metadata()
            .map_err(|e| (e, path.to_string_lossy().to_string()))?
            .is_dir()
        {
            traverse_directory(entry.path(), param_map)?;
            continue;
        }

        let entry_path = entry.path();
        let stem = entry_path
            .strip_prefix(NOISE_PARAMETER_PATH)
            .expect("should've been able to strip prefix");

        let item = stem.to_string_lossy();
        let item = item.strip_suffix(".json").unwrap_or(item.as_ref());
        let item = item.replace("\\", "/");

        let file = fs::read_to_string(&entry_path).map_err(|e| {
            (
                std::io::Error::other(e),
                entry.path().to_string_lossy().to_string(),
            )
        })?;
        let param = serde_json::from_str(&file).map_err(|err| {
            (
                std::io::Error::new(std::io::ErrorKind::InvalidData, err),
                entry.path().to_string_lossy().to_string(),
            )
        })?;

        param_map.insert(item, param);
    }

    Ok(())
}
