use oyakodon::primitive::{Bowl, ProtectedForAll, View};

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

#[test]
fn nested_bowl() {
    let a = String::from("hello");
    let b = String::from("world");
    let bowl = Bowl::new_box(a).map(|view, slot| {
        let view = view.map(&slot, |a, stamp| {
            stamp.stamp::<dyn for<'a> View<
                'a,
                Output = Bowl<_, dyn for<'b> View<'b, Output = (&'a String, &'b String)>>,
            >>(Bowl::new_box(b).map(|view, slot| {
                let view = view.map(&slot, |b, stamp| stamp.stamp((&*a, &*b)));
                slot.fill(view)
            }))
        });
        slot.fill(view)
    });
    let swapped_base = bowl.map(|view, slot_a| {
        type BowlA<'b> = dyn for<'a> View<'a, Output = (&'a String, &'b String)>;
        view.map(&slot_a, |view, stamp_a| {
            view.map(|view, slot_b| {
                let view = view.map(&slot_b, |view, stamp_b| {
                    stamp_b.stamp::<dyn for<'b> View<'b, Output = ProtectedForAll<BowlA<'b>>>>(
                        stamp_a.stamp(view),
                    )
                });
                slot_b.fill(view)
            })
        })
        .map(|view, slot_b| {
            let view = view.map(&slot_b, |view, stamp_b| {
                stamp_b
                    .stamp::<dyn for<'b> View<'b, Output = Bowl<_, BowlA<'b>>>>(slot_a.fill(view))
            });
            slot_b.fill(view)
        })
    });
    swapped_base.map(|view, slot| {
        view.map(&slot, |view, _| {
            let b = view.map(|view, slot| {
                view.map(&slot, |(a, b), _| {
                    assert_eq!(a, "hello");
                    b
                })
            });
            assert_eq!(b, "world");
        })
    });
}
