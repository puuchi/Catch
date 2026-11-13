use catch_core::resources::{validate_list, Command, Kind, Owner, Resource};
#[test]
fn supports_unicode_paths_network_folders_and_http_links_but_no_relative_or_script_targets() {
    for (kind, target) in [
        (Kind::File, r"D:\资料\阅读.pdf"),
        (Kind::Folder, r"\\server\share\材料"),
        (Kind::Web, "https://example.com/阅读?q=1#part"),
    ] {
        assert!(Resource::new("r", kind, target, None).is_ok(), "{target}");
    }
    for (kind, target) in [
        (Kind::File, "relative.txt"),
        (Kind::File, r"C:relative.txt"),
        (Kind::Web, "javascript:alert(1)"),
        (Kind::Web, "file:///D:/a.txt"),
        (Kind::Web, "https://user:secret@example.com/"),
        (Kind::File, "D:\\a\0.txt"),
        (Kind::File, r"\\.\PhysicalDrive0"),
    ] {
        assert!(Resource::new("r", kind, target, None).is_err(), "{target}");
    }
}
#[test]
fn relocation_preserves_identity_and_labels_and_removal_only_changes_the_list() {
    let original = Resource::new("r", Kind::File, r"D:\资料\旧版.pdf", Some("阅读材料")).unwrap();
    let relocated = Command::Relocate {
        owner: Owner::Matter { id: "a".into() },
        resource_id: "r".into(),
        target: r"D:\资料\新版.pdf".into(),
    }
    .apply(std::slice::from_ref(&original), "unused")
    .unwrap();
    assert_eq!(relocated[0].id, "r");
    assert_eq!(relocated[0].title, "阅读材料");
    assert_ne!(relocated[0].target, original.target);
    assert!(Command::Remove {
        owner: Owner::Matter { id: "a".into() },
        resource_id: "r".into()
    }
    .apply(&relocated, "unused")
    .unwrap()
    .is_empty());
}
#[test]
fn same_windows_path_is_not_added_twice_with_different_case_or_slashes() {
    let a = Resource::new("a", Kind::File, r"D:\Materials\A.pdf", None).unwrap();
    let b = Resource::new("b", Kind::File, "d:/materials/a.PDF", None).unwrap();
    assert!(validate_list(&[a, b]).is_err());
}
