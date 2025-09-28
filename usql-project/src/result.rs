pub trait IntoResult {
    type Error;
    type Ok;

    fn into_result(self) -> Result<Self::Ok, Self::Error>;

    fn as_result(&self) -> Result<&Self::Ok, &Self::Error>;
}

impl<T, E> IntoResult for Result<T, E> {
    type Error = E;
    type Ok = T;

    fn into_result(self) -> Result<T, E> {
        self
    }

    fn as_result(&self) -> Result<&T, &E> {
        self.as_ref()
    }
}
