//! An example of using covariant views to get a direct reference to the view
//! without wrapping everything in a function inside [`Bowl::spawn`].

use ::{
    aliasable::boxed::AliasableBox,
    oyakodon::primitive::{Bowl, View},
};

trait Covariant<'ub> {
    type Value<'a>
    where
        'ub: 'a;
    fn as_ref<'a, 'b, 'c>(value: &'c Self::Value<'a>) -> &'c Self::Value<'b>
    where
        'a: 'b;
}

struct SelfRef<'ub, T, F>(
    Bowl<'ub, AliasableBox<T>, dyn for<'x> View<'x, Output = F::Value<'x>> + 'static>,
)
where
    T: ?Sized,
    F: Covariant<'ub>;

impl<'ub, T: ?Sized, F: Covariant<'ub>> SelfRef<'ub, T, F> {
    fn get<'a>(&'a self) -> &'a F::Value<'a>
    where
        F::Value<'a>: 'a,
    {
        self.0.with(|view, _| F::as_ref(view))
    }
}

struct StrRef;
impl<'ub> Covariant<'ub> for StrRef {
    type Value<'a>
        = &'a str
    where
        'ub: 'a;
    fn as_ref<'a, 'b, 'c>(value: &'c &'a str) -> &'c &'b str
    where
        'a: 'b,
    {
        // &'a str is covariant in 'a, so &'a str: &'b str when 'a: 'b.
        // Rust applies lifetime subtyping here automatically.
        value
    }
}

fn main() {
    let cell = SelfRef::<_, StrRef>(
        Bowl::new_box(String::from("hello, world")).map(|view, slot| slot.fill(view.as_str())),
    );
    let view = cell.get();
    println!("{view}");
}
