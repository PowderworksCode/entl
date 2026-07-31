use std::path::PathBuf;

use serde::Deserialize;

use crate::GithubActionPublicationFacts;

#[derive(Deserialize)]
struct ActionManifest {
    name: String,
}

pub fn inspect_action_publication(
    manifest_path: PathBuf,
    manifest: &str,
    readme_path: Option<PathBuf>,
    readme: Option<&str>,
) -> Result<GithubActionPublicationFacts, String> {
    let manifest = serde_yaml_ng::from_str::<ActionManifest>(manifest)
        .map_err(|error| format!("action manifest is invalid YAML: {error}"))?;
    let slug = marketplace_slug(&manifest.name)
        .ok_or_else(|| "action manifest name cannot form a Marketplace slug".to_owned())?;
    let marketplace_url = format!("https://github.com/marketplace/actions/{slug}");
    let marketplace_linked = readme.is_some_and(|text| {
        text.to_ascii_lowercase()
            .contains(&marketplace_url.to_ascii_lowercase())
    });
    Ok(GithubActionPublicationFacts {
        manifest_path,
        name: manifest.name,
        marketplace_slug: slug,
        marketplace_url,
        readme_path,
        marketplace_linked,
    })
}

fn marketplace_slug(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut separator = false;
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    (!slug.is_empty()).then_some(slug)
}
