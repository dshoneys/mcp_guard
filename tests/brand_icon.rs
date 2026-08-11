use mcp_guard::ui_shell::brand_icon_rgba;

#[test]
fn brand_logo_decodes_to_rgba() {
    let (rgba, w, h) = brand_icon_rgba(32).expect("load brand logo");
    assert_eq!(w, 32);
    assert_eq!(h, 32);
    assert_eq!(rgba.len(), 32 * 32 * 4);
    assert!(rgba.iter().any(|b| *b > 0));
}
