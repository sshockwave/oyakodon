//! An example of using covariant views to get a direct reference to the view
//! without wrapping everything in a function inside [`BowlRef::spawn`].

use oyakodon::{BowlRef, Derive, View};
use std::marker::PhantomData;

trait Covariant<'ub> {
    type Value<'a>;
    fn as_ref<'a, 'b, 'c>(value: &'c Self::Value<'a>) -> &'c Self::Value<'b>
    where
        'a: 'b;
}

struct CovariantView<'ub, F>(PhantomData<&'ub ()>, PhantomData<F>);
impl<'a, 'ub, T: ?Sized, F> View<&'a T> for CovariantView<'ub, F>
where
    F: Covariant<'ub>,
{
    type Output = F::Value<'a>;
}

struct SelfRef<'ub, T, F>(BowlRef<'ub, Box<T>, CovariantView<'ub, F>>)
where
    T: ?Sized,
    F: Covariant<'ub>;

impl<'ub, T: ?Sized, F: Covariant<'ub>> SelfRef<'ub, T, F> {
    fn get<'a>(&'a self) -> &'a F::Value<'a>
    where
        F::Value<'a>: 'a,
    {
        struct Shorten<'ub, F, T: ?Sized>(PhantomData<&'ub ()>, PhantomData<F>, PhantomData<T>);
        impl<'a, 'ub, T: ?Sized, F: Covariant<'ub>> View<&'a F::Value<'_>> for Shorten<'ub, F, T>
        where
            F::Value<'a>: 'a,
        {
            type Output = &'a F::Value<'a>;
        }
        impl<'a, 'b, 'ub, T: ?Sized, F: Covariant<'ub>> Derive<&'a F::Value<'b>, &'a &'b ()>
            for Shorten<'ub, F, T>
        where
            F::Value<'a>: 'a,
        {
            fn call(self, value: &'a F::Value<'b>) -> &'a F::Value<'a> {
                F::as_ref(value)
            }
        }

        self.0
            .spawn(Shorten::<'ub, F, T>(PhantomData, PhantomData, PhantomData))
    }
}

struct StrRef;
impl Covariant<'static> for StrRef {
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
