use oyakodon::primitive::{Bowl, View};

#[test]
fn owner_nonstatic_lifetime() {
    let s = String::from("hello");
    fn work<'a>(s: &'a String) -> Option<char> {
        Bowl::new_box(s).map(|view, _, _| view.chars()).next()
    }
    assert_eq!(work(&s), Some('h'));
}

#[test]
fn nested_bowl() {
    let a = String::from("hello");
    let b = String::from("world");
    let bowl = Bowl::new_box(a).map(|a, slot, stamp| {
        slot.fill::<dyn for<'a> View<
            'a,
            Output = Bowl<_, dyn for<'b> View<'b, Output = (&'a String, &'b String)>>,
        >, _>(
            Bowl::new_box(b).map(|b, slot, stamp| slot.fill((&*a, &*b), stamp)),
            stamp,
        )
    });
    let swapped_base = bowl.map(|view, slot_a, stamp_a| {
        type BowlA<'b> = dyn for<'a> View<'a, Output = (&'a String, &'b String)>;
        view.map(|view, slot_b, stamp_b| {
            slot_b.fill::<dyn for<'b> View<'b, Output = Bowl<_, BowlA<'b>>>, _>(
                slot_a.fill(view, stamp_a),
                stamp_b,
            )
        })
    });
    swapped_base.map(|view, _, _| {
        let b = view.map(|(a, b), _, _| {
            assert_eq!(a, "hello");
            b
        });
        assert_eq!(b, "world");
    });
}
