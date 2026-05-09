pub trait View<'x> {
    type Output;
}

#[macro_export]
macro_rules! bounded_view {
    (
        $(#[$meta:meta])*
        $pub:vis trait $name:ident {}
    ) => {
        $(#[$meta])*
        $pub trait $name<'x, 'ub, X = &'x &'ub ()>:
            $crate::View<'x, Output = Self::Target>
        {
            type Target;
        }
        impl<'x, T: ?Sized> $name<'x, '_> for T
        where
            T: $crate::View<'x>,
        {
            /// A different name is used to avoid conflicts with [`View::Output`].
            type Target = Self::Output;
        }
    };
}

bounded_view!(
    /// Helper to create higher-kinded `Lifetime -> Type` with an upper bound on the lifetime.
    ///
    /// Current Rust does not allow lifetime bounds in HRTB,
    /// but it allows you to have implicit bounds.
    /// This is a subtle feature of Rust
    /// without which it would be very feature-incomplete.
    /// ```
    /// trait Bounded<'x, 'ub, X = &'x &'ub ()> {
    ///   // ...
    /// }
    /// ```
    /// Here, `X` is where the magic happens:
    /// for `&'x &'ub ()` to be well-formed,
    /// `'ub` must outlive `'x`,
    /// and thus any expression containing `Bounded` must satisfy this constraint.
    /// This allows us to write a trait bound `T: for<'x> Bounded<'x, 'ub>`
    /// that only requires `T` to be implemented for any `'x` shorter than `'ub`.
    ///
    /// Read more on this in
    /// _[The Better Alternative to Lifetime GATs](https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats)_.
    ///
    /// # How about lower bounds?
    ///
    /// We don't directly provide a trait `TwoSidedBound<'x, 'lb, 'ub>`
    /// because lower bounds do not have a single infimum like `'static` for upper bounds,
    /// so unspecified lower bounds in this trait need to be written as
    /// `for<'lb, 'x> TwoSidedBound<'x, 'lb, 'ub>`.
    /// This complicates our current use cases which only require upper bounds.
    /// Nevertheless, it can be achieved with `BoundedView<'x, 'ub, &'lb &'x &'ub ()>`.
    ///
    /// # `T: for<'x> BoundedView<'x, 'long>` does not imply `T: for<'x> BoundedView<'x, 'short>`
    ///
    /// This can be overcome by creating a second `BoundedView` trait
    /// or materializing `T` to `dyn for<'x> View<'x>`.
    /// We recommend using the `dyn` syntax for uniformity,
    /// but you can create your own `BoundedView` trait by copy-pasting
    /// ```
    /// use oyakodon::View;
    /// pub trait BoundedView<'x, 'ub, X: ?Sized = &'x &'ub ()>: View<'x, Output = Self::Target> {
    ///     type Target;
    /// }
    /// impl<'x, T, X: ?Sized> BoundedView<'x, '_, X> for T
    /// where
    ///     T: View<'x> + ?Sized,
    /// {
    ///     type Target = Self::Output;
    /// }
    /// ```
    /// Or even better, use the provided `bounded_view!` macro to generate it for you:
    /// ```
    /// use oyakodon::bounded_view;
    /// bounded_view!(
    ///     pub trait MyBoundedView {}
    /// );
    /// ```
    /// Note that requiring both `T: for<'x> BoundedView<'x, 'short> + for<'x> BoundedView<'x, 'long>`
    /// does not work in current Rust since it cannot disambiguate the associated type `Target`.
    pub trait BoundedView {}
);
