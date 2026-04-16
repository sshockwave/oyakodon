#[test]
fn readme_basic() {
    use oyakodon::BowlBox;

    fn parse_words(s: &mut String) -> Vec<&str> {
        s.split_whitespace().collect()
    }

    let mut view = BowlBox::new("hello world foo".to_owned(), parse_words);
    view.get_mut()[2] = "bar";
    assert_eq!(*view.get(), vec!["hello", "world", "bar"]);

    assert_eq!(view.into_owner(), "hello world foo");
}

#[test]
#[cfg_attr(miri, ignore)]
fn readme_monadic() {
    use oyakodon::BowlBox;

    fn parse_and_double(
        s: &mut String,
    ) -> Result<std::future::Ready<i32>, std::num::ParseIntError> {
        Ok(std::future::ready(s.trim().parse::<i32>()? * 2))
    }

    let result = smol::block_on(async {
        BowlBox::new("21".to_owned(), parse_and_double)
            .into_result() // Result<BowlBox<...>, BowlBox<...>>
            .unwrap() // BowlBox<Ready<i32>>
            .into_async() // impl Future<Output = BowlBox<i32>>
            .await // BowlBox<i32>
            .into_view() // i32
    });
    assert_eq!(result, 42);
}

#[test]
fn readme_closure() {
    use oyakodon::{BowlBox, View};

    struct Word;
    impl<'a> View<&'a mut String> for Word {
        type Output = &'a str;
    }

    let nth_word = 1;
    let view = BowlBox::<_, Word>::from_fn("hello world foo".to_owned(), &|s| {
        s.split_whitespace().nth(nth_word).unwrap_or("")
    });
    assert_eq!(*view.get(), "world");
}

#[test]
fn readme_derive() {
    use oyakodon::{BowlBox, Derive, View};

    struct NthWord(usize);
    impl<'a> View<&'a mut String> for NthWord {
        type Output = &'a str;
    }
    impl<'a> Derive<&'a mut String> for NthWord {
        fn call(self, s: &'a mut String) -> &'a str {
            s.split_whitespace().nth(self.0).unwrap_or("")
        }
    }

    let view = BowlBox::new("hello world foo".to_owned(), NthWord(1));
    assert_eq!(*view.get(), "world");
}

#[test]
fn readme_shared() {
    use oyakodon::BowlRef;
    use std::rc::Rc;

    fn parse_words(s: &String) -> Vec<&str> {
        s.split_whitespace().collect()
    }

    let view = BowlRef::new(Rc::new("hello world foo".to_owned()), parse_words);
    assert_eq!(*view.get(), vec!["hello", "world", "foo"]);

    let _view = view.clone();
}

#[test]
fn readme_cast() {
    use oyakodon::{BowlBox, View};

    // Two different view marker types that both produce `usize`
    fn str_len(s: &mut String) -> usize {
        s.len()
    }

    struct Len;
    impl<'a> View<&'a mut String> for Len {
        type Output = usize;
    }

    let a = BowlBox::new("hello".to_owned(), str_len);
    let b: BowlBox<_, Len> = a.cast();
    assert_eq!(*b.get(), 5);
}
