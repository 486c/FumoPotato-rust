use fumo_macro::listing;

#[listing]
struct Foo {
    x: i64,
}

#[test]
fn test_listing_builder() {
    let mut f = Foo::new(23).max_pages(1);

    assert_eq!(f.x, 23);

    f.next_page();
    f.next_page();
    f.next_page();

    assert_eq!(f.current_page, 1);
}

#[test]
fn test_calculate_pages() {
    let f = Foo::new(1337).calculate_pages(10, 3);

    assert_eq!(f.entries_per_page, 3);
    assert_eq!(f.max_pages, 4);

    let f = Foo::new(1337).calculate_pages(50, 20);

    assert_eq!(f.entries_per_page, 20);
    assert_eq!(f.max_pages, 3);
}
