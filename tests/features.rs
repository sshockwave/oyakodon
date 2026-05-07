use oyakodon::primitive::{Bowl, View};

#[test]
fn owner_nonstatic_lifetime() {
    let s = String::from("hello");
    fn work<'a>(s: &'a String) -> Option<char> {
        Bowl::new_box(s).map(|view, _| view.chars()).next()
    }
    assert_eq!(work(&s), Some('h'));
}

#[test]
fn nested_bowl() {
    let a = String::from("hello");
    let b = String::from("world");
    let bowl = Bowl::new_box(a).map(|a, slot| {
        slot.fill::<dyn for<'a> View<
            'a,
            Output = Bowl<_, dyn for<'b> View<'b, Output = (&'a String, &'b String)>>,
        >>(Bowl::new_box(b).map(|b, slot| slot.fill((&*a, &*b))))
    });
    let swapped_base = bowl.map(|view, slot_a| {
        type BowlA<'b> = dyn for<'a> View<'a, Output = (&'a String, &'b String)>;
        view.map(|view, slot_b| {
            slot_b.fill::<dyn for<'b> View<'b, Output = Bowl<_, BowlA<'b>>>>(slot_a.fill(view))
        })
    });
    swapped_base.map(|view, _| {
        let b = view.map(|(a, b), _| {
            assert_eq!(a, "hello");
            b
        });
        assert_eq!(b, "world");
    });
}
