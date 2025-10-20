use pexels::output::parse_page_number;

#[test]
fn test_parse_page_number() {
    assert_eq!(parse_page_number("https://x/y?page=2&per_page=5"), Some(2));
    assert_eq!(parse_page_number("/v1/search?page=10"), Some(10));
    assert_eq!(parse_page_number("/v1/search?foo=bar"), None);
}

