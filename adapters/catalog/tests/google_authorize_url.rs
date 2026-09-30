//! Google writes `https://accounts.google.com/o/oauth2/auth` as the `auth_uri`
//! of a downloaded installed-app client file, and the client-file connect
//! compares the profile's `authorize_url` with it byte for byte, so a guide
//! configuration naming the `/o/oauth2/v2/auth` endpoint is refused at connect.
//! The check walks every `docs/catalog-google-*.md`, so a Google guide added
//! later is covered without being named here, and the catalog provider guide.
use std::{fs, path::Path};

const REFUSED: &str = "/o/oauth2/v2/auth";

#[test]
fn no_google_guide_names_the_v2_authorize_endpoint() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let mut checked = Vec::new();
    let mut naming = Vec::new();
    for entry in fs::read_dir(&docs).unwrap() {
        let name = entry.unwrap().file_name().to_str().unwrap().to_owned();
        let google = name.starts_with("catalog-google-") && name.ends_with(".md");
        if !google && name != "local-catalog-provider.md" {
            continue;
        }
        if fs::read_to_string(docs.join(&name))
            .unwrap()
            .contains(REFUSED)
        {
            naming.push(name.clone());
        }
        checked.push(name);
    }
    checked.sort();
    for guide in [
        "catalog-google-calendar.md",
        "catalog-google-drive.md",
        "catalog-google-gmail.md",
        "catalog-google-slides.md",
        "local-catalog-provider.md",
    ] {
        assert!(checked.iter().any(|c| c == guide), "{guide}: {checked:?}");
    }
    assert!(
        naming.is_empty(),
        "these guides name {REFUSED}, which a downloaded client file's auth_uri \
         (https://accounts.google.com/o/oauth2/auth) never equals: {naming:?}"
    );
}
