pub use wit_bindgen_host_wasmtime_rust_macro::*;

#[cfg(feature = "tracing-lib")]
pub use tracing_lib as tracing;
#[doc(hidden)]
pub use {anyhow, async_trait::async_trait, wasmtime};

pub type Result<A, E> = std::result::Result<A, Error<E>>;

pub struct Error<T> {
    err: anyhow::Error,
    ty: std::marker::PhantomData<T>,
}

impl<T: std::error::Error + Send + Sync + 'static> Error<T> {
    pub fn new(err: T) -> Error<T> {
        Error {
            err: err.into(),
            ty: std::marker::PhantomData,
        }
    }

    pub fn into_inner(self) -> anyhow::Error {
        self.err
    }

    pub fn context<C>(self, context: C) -> Error<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.err.context(context).into()
    }

    pub fn downcast(self) -> std::result::Result<T, Error<T>> {
        match self.err.downcast::<T>() {
            Ok(t) => Ok(t),
            Err(err) => Err(Self { err, ty: self.ty }),
        }
    }

    pub fn downcast_ref(&self) -> Option<&T> {
        self.err.downcast_ref::<T>()
    }

    pub fn downcast_mut(&mut self) -> Option<&mut T> {
        self.err.downcast_mut::<T>()
    }
}

impl<T> std::ops::Deref for Error<T> {
    type Target = dyn std::error::Error + Send + Sync + 'static;
    fn deref(&self) -> &Self::Target {
        self.err.deref()
    }
}
impl<T> std::ops::DerefMut for Error<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.err.deref_mut()
    }
}

impl<T> std::fmt::Display for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.err.fmt(f)
    }
}

impl<T> std::fmt::Debug for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.err.fmt(f)
    }
}

impl<T> std::error::Error for Error<T> {}

impl<T> From<anyhow::Error> for Error<T> {
    fn from(err: anyhow::Error) -> Error<T> {
        Error {
            err,
            ty: std::marker::PhantomData,
        }
    }
}
