use oyakodon::primitive::{Bowl, View};

#[test]
fn owner_nonstatic_lifetime() {
    let s = String::from("hello");
    fn work<'a>(s: &'a String) -> Option<char> {
        Bowl::new_box(s)
            .map(|view, _| view.map(|view, _| view.chars()))
            .next()
    }
    assert_eq!(work(&s), Some('h'));
}

#[test]
fn nested_bowl() {
    let a = String::from("hello");
    let b = String::from("world");
    let bowl = Bowl::new_box(a).map(|view, slot| {
        let view = view.map(|a, stamp| {
            stamp.stamp::<dyn for<'a> View<
                'a,
                Output = Bowl<_, dyn for<'b> View<'b, Output = (&'a String, &'b String)>>,
            >>(Bowl::new_box(b).map(|view, slot| {
                let view = view.map(|b, stamp| stamp.stamp((&*a, &*b)));
                slot.fill(view)
            }))
        });
        slot.fill(view)
    });
    let swapped_base = bowl.map(|view, slot_a| {
        type BowlA<'b> = dyn for<'a> View<'a, Output = (&'a String, &'b String)>;
        view.map(|view, stamp_a| {
            view.map(|view, slot_b| {
                view.map(|view, stamp_b| {
                    slot_b.fill(
                        stamp_b.stamp::<dyn for<'b> View<'b, Output = Bowl<_, BowlA<'b>>>>(
                            slot_a.fill(stamp_a.stamp(view)),
                        ),
                    )
                })
            })
        })
    });
    swapped_base.map(|view, _| {
        view.map(|view, _| {
            let b = view.map(|view, _| {
                view.map(|(a, b), _| {
                    assert_eq!(a, "hello");
                    b
                })
            });
            assert_eq!(b, "world");
        })
    });
}
