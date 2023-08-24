//! Provides FFI-safe abstractions

use core::ffi::FromBytesUntilNulError;
use std::{
	cmp::Ordering,
	ffi::{c_char, CStr, CString, NulError},
	fmt::{self, Display, Formatter},
	marker::PhantomData,
	ops::{Deref, DerefMut},
	slice::{Iter, IterMut},
};
use time::Time;

/// FFI-safe [`slice`]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiSlice<'t, T> {
	/// Pointer to the data
	ptr: *const T,
	/// Number of elements contained in the slice
	len: usize,
	/// Phantom to bind the generics
	_phantom: PhantomData<&'t [T]>,
}
impl<'t, T> FfiSlice<'t, T> {
	/// Constructs a new instance
	#[inline]
	pub const fn new(slice: &'t [T]) -> Self {
		Self {
			ptr: slice.as_ptr(),
			len: slice.len(),
			_phantom: PhantomData,
		}
	}

	/// Returns the number of elements contained in the slice
	#[inline]
	pub const fn len(&self) -> usize {
		self.len
	}

	/// Checks if the slice is empty
	#[inline]
	pub const fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Constructs back a slice
	#[inline]
	pub const fn to_slice(&self) -> &'t [T] {
		// SAFETY: This struct can only be constructed from a `&[T]`,
		// and there is no way to get the ownership of the data.
		unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
	}

	/// Returns an [`Iterator`] over the elements contained in the slice
	#[inline]
	pub fn iter(&self) -> Iter<'_, T> {
		self.into_iter()
	}
}
impl<'t, T> From<&'t [T]> for FfiSlice<'t, T> {
	#[inline]
	fn from(slice: &'t [T]) -> Self {
		Self::new(slice)
	}
}
impl<'t, T> Deref for FfiSlice<'t, T> {
	type Target = [T];

	#[inline]
	fn deref(&self) -> &Self::Target {
		self.to_slice()
	}
}
impl<'t, T> IntoIterator for &'t FfiSlice<'t, T> {
	type Item = &'t T;
	type IntoIter = Iter<'t, T>;

	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.deref().iter()
	}
}

/// FFI-safe [`Box<[T]>`]
#[repr(C)]
#[derive(Debug)]
pub struct FfiBoxedSlice<T> {
	/// Pointer to the data
	ptr: *mut T,
	/// Number of elements contained in the slice
	len: usize,
}
impl<T> FfiBoxedSlice<T> {
	/// Constructs a new instance
	#[inline]
	pub fn new(slice: Box<[T]>) -> Self {
		Self {
			len: slice.len(),
			ptr: Box::into_raw(slice).cast(),
		}
	}

	/// Returns the number of elements contained in the slice
	#[inline]
	pub const fn len(&self) -> usize {
		self.len
	}

	/// Checks if the slice is empty
	#[inline]
	pub const fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Returns an FFI-safe slice
	#[inline]
	pub const fn as_slice(&self) -> FfiSlice<'_, T> {
		FfiSlice {
			ptr: self.ptr.cast_const(),
			len: self.len,
			_phantom: PhantomData,
		}
	}

	/// Constructs back a slice
	#[inline]
	pub const fn to_slice(&self) -> &[T] {
		// SAFETY: This struct can only be constructed from a `Box<[T]>`,
		// and there is no way to get the ownership of the data.
		unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
	}
}
impl<T: Clone> Clone for FfiBoxedSlice<T> {
	#[inline]
	fn clone(&self) -> Self {
		Self::new(Box::from(self.to_slice()))
	}
}
impl<T> From<Box<[T]>> for FfiBoxedSlice<T> {
	#[inline]
	fn from(slice: Box<[T]>) -> Self {
		Self::new(slice)
	}
}
impl<T> Deref for FfiBoxedSlice<T> {
	type Target = [T];

	#[inline]
	fn deref(&self) -> &Self::Target {
		self.to_slice()
	}
}
impl<T> DerefMut for FfiBoxedSlice<T> {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		// SAFETY: This struct can only be constructed from a `Box<[T]>`,
		// and there is no way to get the ownership of the data.
		unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
	}
}
impl<T> Drop for FfiBoxedSlice<T> {
	#[inline]
	fn drop(&mut self) {
		// SAFETY: This struct can only be constructed from a `Box<[T]>`,
		// and there is no way to get the ownership of the data.
		drop(unsafe { Box::from_raw(self.deref_mut()) });
	}
}
impl<T> FromIterator<T> for FfiBoxedSlice<T> {
	#[inline]
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		Box::<[T]>::from_iter(iter).into()
	}
}
impl<'slice, T> IntoIterator for &'slice FfiBoxedSlice<T> {
	type Item = &'slice T;
	type IntoIter = Iter<'slice, T>;

	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.deref().iter()
	}
}
impl<'slice, T> IntoIterator for &'slice mut FfiBoxedSlice<T> {
	type Item = &'slice mut T;
	type IntoIter = IterMut<'slice, T>;

	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.deref_mut().iter_mut()
	}
}

/// FFI-safe [`str`]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiStr<'s> {
	/// Pointer to the data
	ptr: *const c_char,
	/// Phantom to bind the generics
	_phantom: PhantomData<&'s str>,
}
impl<'s> FfiStr<'s> {
	/// Constructs a new instance
	///
	/// # Errors
	/// This function returns a [`FromBytesWithNulError`]
	/// if the given string is not nul-terminated
	/// or contains interior nul bytes.
	#[inline]
	pub const fn new(s: &'s str) -> Result<Self, FromBytesUntilNulError> {
		match CStr::from_bytes_until_nul(s.as_bytes()) {
			Ok(cstr) => Ok(Self {
				ptr: cstr.as_ptr(),
				_phantom: PhantomData,
			}),
			Err(err) => Err(err),
		}
	}

	/// Constructs back a string slice
	#[inline]
	pub fn to_str(&self) -> &'_ str {
		// SAFETY: This struct can only be constructed from a `&str`,
		// and there is no way to get the ownership of the data.
		unsafe { std::str::from_utf8_unchecked(CStr::from_ptr(self.ptr).to_bytes()) }
	}
}
impl<'s> TryFrom<&'s str> for FfiStr<'s> {
	type Error = FromBytesUntilNulError;

	#[inline]
	fn try_from(s: &'s str) -> Result<Self, Self::Error> {
		Self::new(s)
	}
}
impl<'s> From<&'s CStr> for FfiStr<'s> {
	#[inline]
	fn from(s: &'s CStr) -> Self {
		Self {
			ptr: s.as_ptr(),
			_phantom: PhantomData,
		}
	}
}
impl<'s> Deref for FfiStr<'s> {
	type Target = str;

	#[inline]
	fn deref(&self) -> &Self::Target {
		self.to_str()
	}
}
impl<'s> Display for FfiStr<'s> {
	#[inline]
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		Display::fmt(self.deref(), f)
	}
}

/// Constructs a new [`FfiStr`] from a string literal
#[macro_export]
macro_rules! new_ffistr {
	($s:expr) => {
		match $crate::ffi::FfiStr::new(::core::concat!($s, '\0')) {
			Ok(s) => s,
			Err(_err) => ::core::unreachable!(),
		}
	};
}
pub use new_ffistr;

/// FFI-safe [`String`]
#[repr(C)]
#[derive(Debug)]
pub struct FfiString {
	/// Pointer to the data
	ptr: *mut c_char,
}
impl FfiString {
	/// Constructs a new instance
	///
	/// # Errors
	/// This function returns a [`NulError`]
	/// if the given string contains a nul byte.
	#[inline]
	pub fn new(s: String) -> Result<Self, NulError> {
		CString::new(s).map(|cstr| Self {
			ptr: cstr.into_raw(),
		})
	}

	/// Returns an FFI-safe string slice
	#[inline]
	pub const fn as_str(&self) -> FfiStr<'_> {
		FfiStr {
			ptr: self.ptr.cast_const(),
			_phantom: PhantomData,
		}
	}

	/// Constructs back a string slice
	#[inline]
	pub fn to_str(&self) -> &str {
		// SAFETY: This struct can only be constructed from a `String`,
		// and there is no way to get the ownership of the data.
		unsafe { std::str::from_utf8_unchecked(CStr::from_ptr(self.ptr).to_bytes()) }
	}
}
impl Clone for FfiString {
	#[inline]
	fn clone(&self) -> Self {
		Self::new(self.to_str().to_owned()).unwrap_or_else(|_err| unreachable!())
	}
}
impl TryFrom<String> for FfiString {
	type Error = NulError;

	#[inline]
	fn try_from(s: String) -> Result<Self, Self::Error> {
		Self::new(s)
	}
}
impl TryFrom<Box<str>> for FfiString {
	type Error = <Self as TryFrom<String>>::Error;

	#[inline]
	fn try_from(s: Box<str>) -> Result<Self, Self::Error> {
		Self::try_from(String::from(s))
	}
}
impl Deref for FfiString {
	type Target = str;

	#[inline]
	fn deref(&self) -> &Self::Target {
		self.to_str()
	}
}
impl Display for FfiString {
	#[inline]
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		Display::fmt(self.deref(), f)
	}
}
impl Drop for FfiString {
	#[inline]
	fn drop(&mut self) {
		// SAFETY: This struct can only be constructed from a `String`,
		// and there is no way to get the ownership of the data.
		drop(unsafe { CString::from_raw(self.ptr) });
	}
}

/// FFI-safe [`Time`]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FfiTime {
	/// [`Time::hour`]
	hour: u8,
	/// [`Time::minute`]
	minute: u8,
	/// [`Time::second`]
	second: u8,
}
impl From<Time> for FfiTime {
	#[inline]
	fn from(time: Time) -> Self {
		let (hour, minute, second) = time.as_hms();
		Self {
			hour,
			minute,
			second,
		}
	}
}
impl From<FfiTime> for Time {
	#[inline]
	fn from(time: FfiTime) -> Self {
		Self::from_hms(time.hour, time.minute, time.second).unwrap_or_else(|_err| unreachable!())
	}
}
impl PartialOrd for FfiTime {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
impl Ord for FfiTime {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering {
		self.hour
			.cmp(&other.hour)
			.then_with(|| self.minute.cmp(&other.minute))
			.then_with(|| self.second.cmp(&other.second))
	}
}

/// FFI-safe [`Option`]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum FfiOption<T> {
	/// [`Option::Some`]
	Some(T),
	/// [`Option::None`]
	None,
}
impl<T> From<Option<T>> for FfiOption<T> {
	#[inline]
	fn from(option: Option<T>) -> Self {
		match option {
			Some(value) => Self::Some(value),
			None => Self::None,
		}
	}
}
impl<T> From<T> for FfiOption<T> {
	#[inline]
	fn from(value: T) -> Self {
		Self::Some(value)
	}
}
impl<T> From<FfiOption<T>> for Option<T> {
	#[inline]
	fn from(option: FfiOption<T>) -> Self {
		match option {
			FfiOption::Some(value) => Some(value),
			FfiOption::None => None,
		}
	}
}

/// FFI-safe [`Result`]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum FfiResult<T, E> {
	/// [`Result::Ok`]
	Ok(T),
	/// [`Result::Err`]
	Err(E),
}
impl<T, E> From<Result<T, E>> for FfiResult<T, E> {
	#[inline]
	fn from(result: Result<T, E>) -> Self {
		match result {
			Ok(value) => Self::Ok(value),
			Err(err) => Self::Err(err),
		}
	}
}
impl<T, E> From<T> for FfiResult<T, E> {
	#[inline]
	fn from(value: T) -> Self {
		Self::Ok(value)
	}
}
impl<T, E> From<FfiResult<T, E>> for Result<T, E> {
	#[inline]
	fn from(result: FfiResult<T, E>) -> Self {
		match result {
			FfiResult::Ok(value) => Ok(value),
			FfiResult::Err(err) => Err(err),
		}
	}
}
