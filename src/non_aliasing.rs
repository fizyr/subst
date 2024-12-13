use core::mem::MaybeUninit;

/// Simple wrapper around [`MaybeUninit`] that guarantees the type is initialized.
///
/// This may sound odd, but the compiler can not assume that `MaybeUninit` is initialized,
/// but as far is the compiler is concerned, `MaybeUninit<T>` is not keeping any references around, even if `T` would.
/// So `NonAliasing<T>` is not aliassing anything untill you call `inner()` to get your `&T` back.
///
/// We use this to create a (hopefully) sound self referential struct in `TemplateBuf` and `ByteTemplateBuf`.
pub struct NonAliasing<T> {
	inner: MaybeUninit<T>,
}

impl<T> NonAliasing<T> {
	pub fn new(inner: T) -> Self {
		let inner = MaybeUninit::new(inner);
		Self { inner }
	}

	pub fn inner(&self) -> &T {
		// SAFETY: We always initialize `inner` in the constructor.
		unsafe {
			self.inner.assume_init_ref()
		}
	}
}

impl<T> Drop for NonAliasing<T> {
	fn drop(&mut self) {
		// SAFETY: We always initialize `inner` in the constructor,
		// the API only exposes `assume_init_ref()`,
		// and we're in the destructor, so nobody is going to call assume_init_read() again.
		unsafe {
			drop(self.inner.assume_init_read())
		}
	}
}
