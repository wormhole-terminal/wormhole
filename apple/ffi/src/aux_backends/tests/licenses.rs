use crate::aux_backends::licenses;

#[test]
fn the_bundled_list_is_complete_and_shares_its_texts_by_index() {
    let list = licenses();
    assert!(list.packages.len() > 100, "the crates Wormhole is built from");
    for package in &list.packages {
        for &index in &package.texts {
            assert!((index as usize) < list.texts.len(), "{} points at a missing text", package.name);
        }
    }
    let shared = list.packages.iter().flat_map(|p| &p.texts).count();
    assert!(list.texts.len() < shared, "identical texts are stored once");
    assert!(list.packages.iter().any(|p| p.is_font), "the bundled font is listed too");
}
