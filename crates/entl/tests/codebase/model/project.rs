// Tests for `src/codebase/model/project.rs`.
//
// A project's three predicates are how every consumer asks what it is made of,
// and each answers about a set that is built elsewhere. What they promise is
// that the question is asked by name rather than by index.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn a_projects_languages_ecosystems_and_facets_are_asked_by_name() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n",
    );
    write(
        temp.path(),
        "src/lib.rs",
        "pub fn value() -> bool { true }\n",
    );
    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let project = inventory.projects.first().expect("one project");

    assert!(project.has_language("rust"));
    assert!(!project.has_language("cobol"));
    assert!(project.uses_ecosystem("cargo"));
    assert!(!project.uses_ecosystem("npm"));
    assert!(!project.has_facet("no-such-facet"));
}
