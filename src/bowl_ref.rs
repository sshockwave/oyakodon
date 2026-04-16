use super::*;
use ::{
    core::{
        cmp::{Eq, PartialEq},
        convert::{AsMut, AsRef},
        fmt::Debug,
        future::Future,
        hash::{Hash, Hasher},
        mem::transmute,
        ops::Deref,
        result::Result,
    },
    maybe_dangling::MaybeDangling,
};

/// Stores an owner and a derived shared reference into it.
///
/// The `T` parameter is the owner container (e.g., [`Box<String>`][Box] or [`Rc<String>`][std::rc::Rc]),
/// and `F` is the view marker type that describes the derivation function.
/// The `'a` parameter is a placeholder lifetime that is deduced automatically.
/// If you need to specify it explicitly,
/// perfer choosing the longest possible lifetime satisfying `T: 'a`
/// to minimize the number of distinct types.
/// For owned heap containers, this is usually `'static`.
///
/// # Examples
///
/// ```
/// use oyakodon::BowlRef;
/// use std::rc::Rc;
///
/// fn first_word(s: &String) -> &str {
///     s.split_whitespace().next().unwrap_or("")
/// }
///
/// let bowl = BowlRef::new(Rc::new("hello world".to_owned()), first_word);
/// assert_eq!(*bowl.get(), "hello");
///
/// let bowl2 = bowl.clone(); // works because Rc is cloneable
/// assert_eq!(*bowl.get(), *bowl2.get());
/// ```
pub struct BowlRef<'a, T: Deref, F: for<'b> View<&'b T::Target> + ?Sized> {
    // `owner` will be dropped after `view`.
    // Rust guarantees that fields are dropped in the order of declaration.
    // https://doc.rust-lang.org/reference/destructors.html#r-destructors.operation
    //
    // Both fields are wrapped in `MaybeDangling` for distinct reasons:
    //
    // `view: MaybeDangling<_>` suppresses Tree Borrows reference protection.
    // When `BowlRef` is passed by value to a function,
    // Tree Borrows would normally "protect" any references inside it for the entire call duration,
    // asserting that the memory they point to remains valid.
    // But `BowlRef` owns both `view` (the borrow) and `owner` (the allocation),
    // so dropping `BowlRef` inside the callee frees `owner` while `view` is still considered live.
    // `MaybeDangling` opts out of the `dereferenceable` assumption, suppressing the protector.
    //
    // `owner: MaybeDangling<_>` suppresses Stacked Borrows Unique retag.
    // Box-like types assert Unique ownership over their allocation whenever they are moved
    // (e.g., as a function argument).
    // If `owner` were moved after `view` was computed,
    // the resulting Unique retag would invalidate `view`'s SharedReadWrite tag on the same allocation.
    // Wrapping `owner` in `MaybeDangling` (a union) before calling `derive`
    // means no further Box move occurs after `view` is created.
    pub(crate) view: MaybeDangling<<F as View<&'a T::Target>>::Output>,
    pub(crate) owner: MaybeDangling<T>,
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: StableDeref,
    F: for<'b> Derive<&'b T::Target>,
{
    pub fn new(owner: T, derive: F) -> Self {
        Self::from_derive(owner, derive)
    }
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: StableDeref,
    F: for<'b> View<&'b T::Target> + ?Sized,
{
    /// The primary constructor. All other constructors delegate here.
    ///
    /// `derive` is called exactly once during construction.
    /// The resulting view is stored alongside the owner.
    pub fn from_derive(
        owner: T,
        derive: impl for<'b> Derive<&'b T::Target, Output = <F as View<&'b T::Target>>::Output>,
    ) -> Self {
        let owner = MaybeDangling::new(owner);
        // SAFETY: The lifetime `'a` passed to `derive()` might differ from the actual borrow,
        // but the HRTB requires `derive()` to work uniformly for any lifetime,
        // so `derive()` cannot exploit the length of `'a`.
        // Thus we can assume that `derive()` had received the real lifetime of `*owner`.
        // Now `view` is annotated with a fake lifetime `'a`
        // and the safety of reading `view` is handed off to getters.
        // We ensure that the owner is never accessed until `view` is dropped
        // to satisfy the possible LLVM `noalias` attribute on `owner`.
        let view = derive.call(unsafe { transmute(&**owner) });
        BowlRef {
            owner,
            view: MaybeDangling::new(view),
        }
    }
    /// Constructs a bowl from a closure, accepting a [`&dyn Fn`][Fn] reference.
    ///
    /// Closures on stable Rust cannot express a return type
    /// whose lifetime is derived from an argument lifetime.
    /// Passing the closure through `&dyn Fn` coerces it into a form
    /// that is generic over the borrow lifetime.
    /// See [`Derive`] for an alternative without `dyn` overhead.
    pub fn from_fn<'b>(
        owner: T,
        derive: &'b dyn for<'c> Fn(&'c T::Target) -> <F as View<&'c T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(owner, derive)
    }

    /// Constructs a bowl from a closure, accepting a [`&mut dyn FnMut`][FnMut] reference.
    ///
    /// Analogous to [`from_fn`][Self::from_fn] but accepts a mutable closure.
    pub fn from_fn_mut<'b>(
        owner: T,
        derive: &'b mut dyn for<'c> FnMut(&'c T::Target) -> <F as View<&'c T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(owner, derive)
    }

    /// Constructs a bowl from a closure, accepting a [`Box<dyn FnOnce>`][FnOnce].
    ///
    /// Analogous to [`from_fn`][Self::from_fn] but the closure is consumed on construction.
    #[cfg(feature = "alloc")]
    pub fn from_fn_once(
        owner: T,
        derive: ::alloc::boxed::Box<
            dyn for<'c> FnOnce(&'c T::Target) -> <F as View<&'c T::Target>>::Output,
        >,
    ) -> Self {
        Self::from_derive(owner, derive)
    }

    /// Transforms the current view using `f` and changes the view type to an explicit `G`.
    ///
    /// Unlike [`map`][Self::map], the target view marker type `G` is specified explicitly
    /// rather than being inferred as [`Map<T::Target, F, H>`][Map].
    /// Use this when you need the output type to match a specific existing marker.
    pub fn map_into<'b, G: ?Sized, H>(self, f: H) -> BowlRef<'b, T, G>
    where
        for<'c> H: Derive<<F as View<&'c T::Target>>::Output>,
        for<'c> G:
            View<&'c T::Target, Output = <H as View<<F as View<&'c T::Target>>::Output>>::Output>,
    {
        let Self { owner, view } = self;
        // SAFETY: The HRTB on this method maintains the HRTB invariant on `derive()`.
        BowlRef::<'_, T, G> {
            owner,
            view: MaybeDangling::new(f.call(MaybeDangling::into_inner(view))),
        }
        .cast()
    }

    /// Transforms the current view using `f`, encoding the composition as [`Map<T::Target, F, G>`][Map].
    ///
    /// Use [`map_into`][Self::map_into] if you need to specify an explicit target view type.
    pub fn map<G>(self, f: G) -> BowlRef<'a, T, Map<T::Target, F, G>>
    where
        G: for<'b> Derive<<F as View<&'b T::Target>>::Output>,
    {
        self.map_into(f)
    }
}

impl<'a, 'b, T, F> AsRef<BowlRef<'b, T, F>> for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c T::Target> + ?Sized,
{
    fn as_ref(&self) -> &BowlRef<'b, T, F> {
        // SAFETY: We maintain an HRTB invariant on `derive()`
        // to make sure any `'c` that does not outlive `*owner`
        // corresponds to a valid `F::Output<'c>`.
        // Since `*owner` is heap-allocated and outlives `self`
        // any borrow `'b` of `self` satisfies the HRTB.
        // This is not a contradiction to the possible invariance of `F::Output`.
        // Think of it as if `derive()` was called with the lifetime of the borrow,
        // and we merely used `'a` as a placeholder.
        // The memory layout is the same because lifetime information is erased in runtime.
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F> AsMut<BowlRef<'b, T, F>> for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c T::Target> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlRef<'b, T, F> {
        // SAFETY: Same as `as_ref()`.
        unsafe { transmute(self) }
    }
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b T::Target> + ?Sized,
{
    /// Changes the lifetime placeholder `'a` without modifying the value.
    ///
    /// Because `'a` is only a placeholder, any valid lifetime can be substituted freely.
    /// See the struct documentation for guidance on choosing `'a`.
    pub fn cast_life<'b>(self) -> BowlRef<'b, T, F> {
        // SAFETY: Same as `as_ref()`.
        unsafe { transmute(self) }
    }

    /// Changes the view marker type `F` to any `G` that produces identical output types.
    ///
    /// Two bowls with different view markers but the same `Output` for all lifetimes
    /// have identical representations at runtime and can be freely interconverted.
    pub fn cast_view<G: ?Sized>(self) -> BowlRef<'a, T, G>
    where
        for<'b> G: View<&'b T::Target, Output = <F as View<&'b T::Target>>::Output>,
    {
        let Self { owner, view } = self;
        BowlRef { owner, view }
    }

    /// Combines [`cast_life`][Self::cast_life] and [`cast_view`][Self::cast_view].
    pub fn cast<'b, G: ?Sized>(self) -> BowlRef<'b, T, G>
    where
        for<'c> G: View<&'c T::Target, Output = <F as View<&'c T::Target>>::Output>,
    {
        self.cast_life().cast_view()
    }

    /// Drops the view and returns the owner.
    ///
    /// The view is explicitly dropped before the owner is extracted,
    /// preserving the correct destruction order even when the view's destructor panics.
    pub fn into_owner(self) -> T {
        let Self { owner, view } = self;
        // `owner` must be dropped even if `view`'s drop panics.
        // Miri reports that this is not guaranteed
        // if `view` is dropped implicitly at the end of the function.
        drop(view);
        // SAFETY: `*owner` is not used elsewhere after `view` is dropped.
        MaybeDangling::into_inner(owner)
    }

    /// Drops the owner and returns the view.
    ///
    /// The bound of this function requires the view cannot borrow from `*owner`.
    /// When the view does borrow from the owner, use [`get`][Self::get] instead.
    pub fn into_view<S>(self) -> S
    where
        for<'c> F: View<&'c T::Target, Output = S>,
    {
        let Self { owner, view } = self;
        // Same reason as `into_owner()`
        drop(owner);
        // SAFETY: The HRTB requires `F::Output` to not depend on `owner`.
        MaybeDangling::into_inner(view)
    }

    /// Returns both the owner and the view as a tuple.
    ///
    /// Carries the same `for<'c>` constraint as [`into_view`][Self::into_view]:
    /// the view type must be lifetime-independent of the owner.
    pub fn into_parts<S>(self) -> (T, S)
    where
        for<'c> F: View<&'c T::Target, Output = S>,
    {
        let Self { owner, view } = self;
        // SAFETY: Same as `into_view()`
        (
            MaybeDangling::into_inner(owner),
            MaybeDangling::into_inner(view),
        )
    }

    /// Awaits the current view future and replaces it with the resolved value.
    ///
    /// The owner remains alive for the entire duration of the await,
    /// so the future may safely hold a reference into `*owner`.
    /// The resulting bowl has a view type of [`Async<T::Target, F>`][Async].
    pub async fn into_async(self) -> BowlRef<'a, T, Async<T::Target, F>>
    where
        for<'b> <F as View<&'b T::Target>>::Output: Future,
    {
        let Self { owner, view } = self;
        BowlRef {
            owner,
            view: MaybeDangling::new(MaybeDangling::into_inner(view).await),
        }
    }

    /// Unwraps an [`Outcome`] view, branching into `Ok` or `Err`.
    /// Both branches retain the owner.
    ///
    /// # Examples
    ///
    /// ```
    /// use oyakodon::BowlRef;
    /// use std::rc::Rc;
    ///
    /// fn try_parse(s: &String) -> Result<i32, std::num::ParseIntError> {
    ///     s.parse()
    /// }
    ///
    /// let ok = BowlRef::new(Rc::new("42".to_owned()), try_parse)
    ///     .into_result()
    ///     .unwrap();
    /// assert_eq!(*ok.get(), 42);
    ///
    /// let err = BowlRef::new(Rc::new("abc".to_owned()), try_parse)
    ///     .into_result()
    ///     .unwrap_err();
    /// // The owner is still accessible from the Err branch.
    /// assert_eq!(err.into_owner(), Rc::new("abc".to_owned()));
    /// ```
    pub fn into_result(
        self,
    ) -> Result<BowlRef<'a, T, Success<T::Target, F>>, BowlRef<'a, T, Failure<T::Target, F>>>
    where
        for<'b> <F as View<&'b T::Target>>::Output: Outcome,
    {
        let Self { owner, view } = self;
        use Result::{Err, Ok};
        match MaybeDangling::into_inner(view).get() {
            Ok(v) => Ok(BowlRef {
                owner,
                view: MaybeDangling::new(v),
            }),
            Err(e) => Err(BowlRef {
                owner,
                view: MaybeDangling::new(e),
            }),
        }
    }

    pub fn get(&self) -> &<F as View<&'_ T::Target>>::Output {
        // SAFETY: Reading `view` is safe only if
        // the lifetime passed to `derive()` is shorter than that of `*owner`.
        // Ideally we would like to use the lifetime of the `self` instance
        // because that's the actual lifetime of `*owner`,
        // but we don't know about that yet,
        // so using `'b` is the best we can do.
        let other: &BowlRef<_, F> = self.as_ref();
        &*other.view
    }
    pub fn get_mut(&mut self) -> &mut <F as View<&'_ T::Target>>::Output {
        // SAFETY: Same as `get()`, but for mutable references.
        let other: &mut BowlRef<_, F> = self.as_mut();
        &mut *other.view
    }
}

impl<'a, T, F> Clone for BowlRef<'a, T, F>
where
    T: super::CloneStableDeref,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Clone,
{
    fn clone(&self) -> Self {
        let owner = self.owner.clone();
        // SAFETY: `StableDeref` should guarantee that `*owner` outlives `owner`,
        // so the new `view` will be valid as long as we hold the new `owner`.
        let view = self.view.clone();
        Self { owner, view }
    }
}

#[cfg(feature = "gat")]
impl<'a, T, F> super::Bowl for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b T::Target> + ?Sized,
{
    type Value<'b>
        = <F as View<&'b T::Target>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        BowlRef::get(self)
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        BowlRef::get_mut(self)
    }
}

// These traits are specific to `BowlRef`
// because they require access to `owner`,
// which is not available in `BowlMut`.
impl<'a, 'b, T, F, G> PartialEq<BowlRef<'b, T, G>> for BowlRef<'a, T, F>
where
    T: Deref + PartialEq,
    F: for<'c> View<&'c T::Target> + ?Sized,
    G: for<'c> View<&'c T::Target> + ?Sized,
    for<'c> <F as View<&'c T::Target>>::Output: PartialEq<<G as View<&'c T::Target>>::Output>,
{
    fn eq(&self, other: &BowlRef<'b, T, G>) -> bool {
        // SAFETY: Accessing `owner` is safe because `view` does not have exlusive access to `owner`.
        (*self.owner).eq(&*other.owner) && self.get().eq(other.get())
    }
}

impl<'a, T, F> Eq for BowlRef<'a, T, F>
where
    T: Deref + Eq,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Eq,
{
}

impl<'a, T, F> Hash for BowlRef<'a, T, F>
where
    T: Deref + Hash,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: Same as `PartialEq::eq()`.
        self.owner.hash(state);
        self.get().hash(state);
    }
}

impl<'a, T, F> Debug for BowlRef<'a, T, F>
where
    T: Deref + Debug,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: Same as `PartialEq::eq()`.
        f.debug_struct("BowlRef")
            .field("owner", &*self.owner)
            .field("view", self.get())
            .finish()
    }
}
