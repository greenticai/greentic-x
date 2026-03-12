use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

type Catalog = BTreeMap<String, String>;
type CatalogsByLocale = BTreeMap<String, Catalog>;

fn main() {
    println!("cargo:rerun-if-changed=i18n");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let i18n_dir = manifest_dir.join("i18n");
    let catalogs = load_catalogs(&i18n_dir);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let dest = out_dir.join("embedded_i18n.rs");
    let mut generated = String::from(
        "pub fn load_embedded_catalogs() -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>> {\n    let mut catalogs = std::collections::BTreeMap::new();\n",
    );

    for (locale, catalog) in catalogs {
        let serialized = serde_json::to_string(&catalog).expect("serialize embedded catalog");
        generated.push_str(&format!(
            "    catalogs.insert({locale:?}.to_string(), serde_json::from_str::<std::collections::BTreeMap<String, String>>({serialized:?}).expect(\"embedded locale catalog must be valid\"));\n",
        ));
    }

    generated.push_str("    catalogs\n}\n");
    fs::write(dest, generated).expect("write embedded_i18n.rs");
}

fn load_catalogs(i18n_dir: &Path) -> CatalogsByLocale {
    let mut catalogs = CatalogsByLocale::new();
    visit_catalog_dir(i18n_dir, &mut catalogs);
    catalogs
}

fn visit_catalog_dir(dir: &Path, catalogs: &mut CatalogsByLocale) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("read {} entry: {error}", dir.display()));
        let path = entry.path();

        if path.is_dir() {
            visit_catalog_dir(&path, catalogs);
            continue;
        }

        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if stem == "locales" {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let parsed: Catalog = serde_json::from_str(&raw)
            .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));

        catalogs
            .entry(stem.to_owned())
            .or_default()
            .extend(parsed);
    }
}
