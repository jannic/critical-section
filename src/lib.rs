#![no_std]
#![doc = include_str!("../README.md")]

mod mutex;

use core::marker::PhantomData;

pub use self::mutex::Mutex;

/// Critical section token.
///
/// An instance of this type indicates that the current thread is executing code within a critical
/// section.
#[derive(Clone, Copy, Debug)]
pub struct CriticalSection<'cs> {
    _0: PhantomData<&'cs ()>,
}

impl<'cs> CriticalSection<'cs> {
    /// Creates a critical section token.
    ///
    /// This method is meant to be used to create safe abstractions rather than being directly used
    /// in applications.
    ///
    /// # Safety
    ///
    /// This must only be called when the current thread is in a critical section. The caller must
    /// ensure that the returned instance will not live beyond the end of the critical section.
    ///
    /// The caller must use adequate fences to prevent the compiler from moving the
    /// instructions inside the critical section to the outside of it. Sequentially consistent fences are
    /// suggested immediately after entry and immediately before exit from the critical section.
    ///
    /// Note that the lifetime `'cs` of the returned instance is unconstrained. User code must not
    /// be able to influence the lifetime picked for this type, since that might cause it to be
    /// inferred to `'static`.
    #[inline(always)]
    pub unsafe fn new() -> Self {
        CriticalSection { _0: PhantomData }
    }
}

#[cfg(any(
    all(feature = "restore-state-none", feature = "restore-state-bool"),
    all(feature = "restore-state-none", feature = "restore-state-u8"),
    all(feature = "restore-state-none", feature = "restore-state-u16"),
    all(feature = "restore-state-none", feature = "restore-state-u32"),
    all(feature = "restore-state-none", feature = "restore-state-u64"),
    all(feature = "restore-state-bool", feature = "restore-state-u8"),
    all(feature = "restore-state-bool", feature = "restore-state-u16"),
    all(feature = "restore-state-bool", feature = "restore-state-u32"),
    all(feature = "restore-state-bool", feature = "restore-state-u64"),
    all(feature = "restore-state-u8", feature = "restore-state-u16"),
    all(feature = "restore-state-u8", feature = "restore-state-u32"),
    all(feature = "restore-state-u8", feature = "restore-state-u64"),
    all(feature = "restore-state-u16", feature = "restore-state-u32"),
    all(feature = "restore-state-u16", feature = "restore-state-u64"),
    all(feature = "restore-state-u32", feature = "restore-state-u64"),
))]
compile_error!("You must set at most one of these Cargo features: restore-state-none, restore-state-bool, restore-state-u8, restore-state-u16, restore-state-u32, restore-state-u64");

#[cfg(not(any(
    feature = "restore-state-bool",
    feature = "restore-state-u8",
    feature = "restore-state-u16",
    feature = "restore-state-u32",
    feature = "restore-state-u64"
)))]
type RawRestoreStateInner = ();

#[cfg(feature = "restore-state-bool")]
type RawRestoreStateInner = bool;

#[cfg(feature = "restore-state-u8")]
type RawRestoreStateInner = u8;

#[cfg(feature = "restore-state-u16")]
type RawRestoreStateInner = u16;

#[cfg(feature = "restore-state-u32")]
type RawRestoreStateInner = u32;

#[cfg(feature = "restore-state-u64")]
type RawRestoreStateInner = u64;

// We have RawRestoreStateInner and RawRestoreState so that we don't have to copypaste the docs 5 times.
// In the docs this shows as `pub type RawRestoreState = u8` or whatever the selected type is, because
// the "inner" type alias is private.

/// Raw, transparent "restore state".
///
/// This type changes based on which Cargo feature is selected, out of
/// - `restore-state-none` (default, makes the type be `()`)
/// - `restore-state-bool`
/// - `restore-state-u8`
/// - `restore-state-u16`
/// - `restore-state-u32`
/// - `restore-state-u64`
///
/// See [`RestoreState`].
///
/// User code uses [`RestoreState`] opaquely, critical section implementations
/// use [`RawRestoreState`] so that they can use the inner value.
pub type RawRestoreState = RawRestoreStateInner;

/// Opaque "restore state".
///
/// Implementations use this to "carry over" information between acquiring and releasing
/// a critical section. For example, when nesting two critical sections of an
/// implementation that disables interrupts globally, acquiring the inner one won't disable
/// the interrupts since they're already disabled. The impl would use the restore state to "tell"
/// the corresponding release that it does *not* have to reenable interrupts yet, only the
/// outer release should do so.
///
/// User code uses [`RestoreState`] opaquely, critical section implementations
/// use [`RawRestoreState`] so that they can use the inner value.
#[derive(Clone, Copy, Debug)]
pub struct RestoreState(RawRestoreState);

impl RestoreState {
    /// Create an invalid, dummy  `RestoreState`.
    ///
    /// This can be useful to avoid `Option` when storing a `RestoreState` in a
    /// struct field, or a `static`.
    ///
    /// Note that due to the safety contract of [`acquire`]/[`release`], you must not pass
    /// a `RestoreState` obtained from this method to [`release`].
    pub const fn invalid() -> Self {
        #[cfg(not(any(
            feature = "restore-state-bool",
            feature = "restore-state-u8",
            feature = "restore-state-u16",
            feature = "restore-state-u32",
            feature = "restore-state-u64"
        )))]
        return Self(());

        #[cfg(feature = "restore-state-bool")]
        return Self(false);

        #[cfg(feature = "restore-state-u8")]
        return Self(0);

        #[cfg(feature = "restore-state-u16")]
        return Self(0);

        #[cfg(feature = "restore-state-u32")]
        return Self(0);

        #[cfg(feature = "restore-state-u64")]
        return Self(0);
    }
}

/// Acquire a critical section in the current thread.
///
/// This function is extremely low level. Strongly prefer using [`with`] instead.
///
/// Nesting critical sections is allowed. The inner critical sections
/// are mostly no-ops since they're already protected by the outer one.
///
/// # Safety
///
/// - Each `acquire` call must be paired with exactly one `release` call in the same thread.
/// - `acquire` returns a "restore state" that you must pass to the corresponding `release` call.
/// - `acquire`/`release` pairs must be "properly nested", ie it's not OK to do `a=acquire(); b=acquire(); release(a); release(b);`.
/// - It is UB to call `release` if the critical section is not acquired in the current thread.
/// - It is UB to call `release` with a "restore state" that does not come from the corresponding `acquire` call.
#[inline]
pub unsafe fn acquire() -> RestoreState {
    extern "Rust" {
        fn _critical_section_1_0_acquire() -> RawRestoreState;
    }

    RestoreState(_critical_section_1_0_acquire())
}

/// Release the critical section.
///
/// This function is extremely low level. Strongly prefer using [`with`] instead.
///
/// # Safety
///
/// See [`acquire`] for the safety contract description.
#[inline]
pub unsafe fn release(restore_state: RestoreState) {
    extern "Rust" {
        fn _critical_section_1_0_release(restore_state: RawRestoreState);
    }
    _critical_section_1_0_release(restore_state.0)
}

/// Execute closure `f` in a critical section.
///
/// Nesting critical sections is allowed. The inner critical sections
/// are mostly no-ops since they're already protected by the outer one.
#[inline]
pub fn with<R>(f: impl FnOnce(CriticalSection) -> R) -> R {
    unsafe {
        let restore_state = acquire();
        let r = f(CriticalSection::new());
        release(restore_state);
        r
    }
}

/// Methods required for a critical section implementation.
///
/// This trait is not intended to be used except when implementing a critical section.
///
/// # Safety
///
/// Implementations must uphold the contract specified in [`crate::acquire`] and [`crate::release`].
pub unsafe trait Impl {
    /// Acquire the critical section.
    unsafe fn acquire() -> RawRestoreState;
    /// Release the critical section.
    unsafe fn release(restore_state: RawRestoreState);
}

/// Set the critical section implementation.
///
/// # Example
///
/// ```
/// use critical_section::RawRestoreState;
///
/// struct MyCriticalSection;
/// critical_section::set_impl!(MyCriticalSection);
///
/// unsafe impl critical_section::Impl for MyCriticalSection {
///     unsafe fn acquire() -> RawRestoreState {
///         // ...
///     }
///
///     unsafe fn release(restore_state: RawRestoreState) {
///         // ...
///     }
/// }
///
#[macro_export]
macro_rules! set_impl {
    ($t: ty) => {
        #[no_mangle]
        unsafe fn _critical_section_1_0_acquire() -> $crate::RawRestoreState {
            <$t as $crate::Impl>::acquire()
        }
        #[no_mangle]
        unsafe fn _critical_section_1_0_release(restore_state: $crate::RawRestoreState) {
            <$t as $crate::Impl>::release(restore_state)
        }
    };
}

// Implement critical-section 0.2 primitives in terms of
// the 1.0 ones, if possible.
// Otherwise provide a dummy implementation which fails to compile
// if the 0.2 ones are called.
#[cfg(any(feature = "restore-state-none",))]
mod compat {
    extern "Rust" {
        fn _critical_section_1_0_acquire() -> RawRestoreState;
        fn _critical_section_1_0_release(restore_state: RawRestoreState);
    }
    #[no_mangle]
    unsafe fn _critical_section_acquire() -> u8 {
        _critical_section_1_0_acquire();
        0
    }

    #[no_mangle]
    unsafe fn _critical_section_release(_: u8) {
        _critical_section_1_0_release()
    }
}
#[cfg(any(feature = "restore-state-bool", feature = "restore-state-u8",))]
mod compat {
    extern "Rust" {
        fn _critical_section_1_0_acquire() -> super::RawRestoreState;
        fn _critical_section_1_0_release(restore_state: super::RawRestoreState);
    }
    #[no_mangle]
    unsafe fn _critical_section_acquire() -> u8 {
        _critical_section_1_0_acquire() as u8
    }

    #[no_mangle]
    unsafe fn _critical_section_release(restore_state: u8) {
        _critical_section_1_0_release(restore_state as _)
    }
}

#[cfg(not(any(
    feature = "restore-state-none",
    feature = "restore-state-bool",
    feature = "restore-state-u8",
)))]
mod compat {
    extern "Rust" {
        // This function is declared but never defined
        fn _do_not_use_critical_section_version_0_2();
    }

    // If those functions are never called, they should be removed
    // by the linker, so calling the undefined function
    // _this_function_does_not_exist doesn't hurt.
    //
    // However, if some code uses critical-section 0.2 and references
    // those functions, they'll be linked in and therefore linking
    // fails.

    #[no_mangle]
    unsafe fn _critical_section_acquire() -> u8 {
        _do_not_use_critical_section_version_0_2();
        unreachable!();
    }

    #[no_mangle]
    unsafe fn _critical_section_release(_: u8) {
        _do_not_use_critical_section_version_0_2();
        unreachable!();
    }
}
