use std::ops::Deref;

/// Holds a [`Vec<u8>`] that does not invalidate references, if it is moved
///
/// See [here][morestina] for an explanation, why this is needed.
///
/// [moresetina]: (https://morestina.net/blog/1868/self-referential-types-for-fun-and-profit#What8217s_the_deal_with_AliasableBox_wouldn8217t_Box_work_as_well)
pub(super) struct Bytes {
	ptr: std::ptr::NonNull<u8>,
	len: usize,
	cap: usize,
}

impl Bytes {
	#[inline]
	pub(super) fn from_bytes(mut vec: Vec<u8>) -> Self {
		// SAFETY: Vec::as_mut_ptr always returns a non-null pointer
		let ptr = unsafe { std::ptr::NonNull::new_unchecked(vec.as_mut_ptr()) };
		let len = vec.len();
		let cap = vec.capacity();
		std::mem::forget(vec);
		Self { ptr, len, cap }
	}

	#[inline]
	pub(super) fn into_bytes(mut self) -> Vec<u8> {
		// SAFETY: self is forgotten immediately
		let vec = unsafe { self.reclaim() };
		std::mem::forget(self);
		vec
	}

	/// Restore the Vec from its raw parts
	///
	/// # Safety
	/// The caller must ensure that never more than one Vec is reclaimed at the
	/// same time.
	#[inline]
	unsafe fn reclaim(&mut self) -> Vec<u8> {
		// SAFETY: ptr, len, and cap where derived from a Vec<u8> and never
		// modified, the original Vec was never dropped, therefore all
		// necessary invariants for Vec::from_raw_parts hold.
		unsafe { Vec::from_raw_parts(self.ptr.as_mut(), self.len, self.cap) }
	}
}

impl Clone for Bytes {
	fn clone(&self) -> Self {
		Self::from_bytes(self.to_vec())
	}

	fn clone_from(&mut self, source: &Self) {
		// SAFETY: vec will be forgotten after contents have been copied and
		// parts have been reassigned.
		let mut vec = unsafe { self.reclaim() };
		vec.clear();
		vec.extend_from_slice(source);
		std::mem::forget(std::mem::replace(self, Self::from_bytes(vec)));
	}
}

impl Deref for Bytes {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &[u8] {
		// SAFETY: ptr, len where derived from a Vec<u8> and never modified,
		// the original Vec was never dropped, therefore ptr points to a valid
		// allocation of size len of type u8, that are properly initialized.
		unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
	}
}

impl Drop for Bytes {
	fn drop(&mut self) {
		drop(unsafe { self.reclaim() });
	}
}

unsafe impl Send for Bytes {}
unsafe impl Sync for Bytes {}

/// Holds a [`std::string::String`] that does not invalidate references, if it
/// is moved
#[derive(Clone)]
pub(super) struct String(Bytes);

impl String {
	#[inline]
	pub(super) fn from_string(string: std::string::String) -> Self {
		Self(Bytes::from_bytes(string.into_bytes()))
	}

	#[inline]
	pub(super) fn into_string(self) -> std::string::String {
		// SAFETY: Inner bytes were created from a valid String and never modified
		unsafe { std::string::String::from_utf8_unchecked(self.0.into_bytes()) }
	}
}

impl Deref for String {
	type Target = str;

	#[inline]
	fn deref(&self) -> &str {
		// SAFETY: Inner bytes were created from a valid String and never modified
		unsafe { std::str::from_utf8_unchecked(&self.0) }
	}
}
