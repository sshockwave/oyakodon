use oyakodon::primitive::Bowl;

#[test]
fn owner_nonstatic_lifetime() {
    let s = String::from("hello");
    fn work<'a>(s: &'a String) -> Option<char> {
        Bowl::new_box(s)
            .map(|view, slot| view.map(&slot, |view, _| view.chars()))
            .next()
    }
    assert_eq!(work(&s), Some('h'));
}
