//! An example of using covariant views to get a direct reference to the view
//! without wrapping everything in a function inside [`BowlRef::spawn`].

use oyakodon::{BowlRef, Derive, View};
use std::marker::PhantomData;

trait Covariant {
    type Value<'a>;
    fn as_ref<'a, 'b, 'c>(value: &'c Self::Value<'a>) -> &'c Self::Value<'b>
    where
        'a: 'b;
}

struct CovariantView<F>(PhantomData<F>);
impl<'a, T: ?Sized, F> View<&'a T> for CovariantView<F>
where
    F: Covariant,
{
    type Output = F::Value<'a>;
}

struct SelfRef<T, F>(BowlRef<'static, Box<T>, CovariantView<F>>)
where
    T: ?Sized + 'static,
    F: Covariant;

impl<T: ?Sized, F: Covariant> SelfRef<T, F> {
    fn get<'a>(&'a self) -> &'a F::Value<'a>
    where
        F::Value<'a>: 'a,
    {
        struct Shorten<F, T: ?Sized>(PhantomData<F>, PhantomData<T>);
        impl<'a, T: ?Sized, F: Covariant> View<&'a F::Value<'_>> for Shorten<F, T>
        where
            F::Value<'a>: 'a,
        {
            type Output = &'a F::Value<'a>;
        }
        impl<'a, 'b, T: ?Sized, F: Covariant> Derive<&'a F::Value<'b>, &'a &'b ()> for Shorten<F, T>
        where
            F::Value<'a>: 'a,
        {
            fn call(self, value: &'a F::Value<'b>) -> &'a F::Value<'a> {
                F::as_ref(value)
            }
        }

        self.0.spawn(Shorten::<F, T>(PhantomData, PhantomData))
    }
}

struct StrRef;
impl Covariant for StrRef {
    type Value<'a> = &'a str;
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
    let cell = SelfRef(BowlRef::<Box<String>, CovariantView<StrRef>>::from_derive(
        Box::new(String::from("hello, world")),
        String::as_str,
    ));
    let view = cell.get();
    println!("{view}");
}
