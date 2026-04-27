pub unsafe trait AliasableDeref {
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

pub unsafe trait AliasableDerefMut: AliasableDeref {
    fn deref_mut(&mut self) -> &mut Self::Target;
}
