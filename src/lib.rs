#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]
#![doc = include_str!("../README.md")]

pub use bare_metal::CriticalSection;

/// Execute closure `f` in a critical section.
///
/// Nesting critical sections is allowed. The inner critical sections
/// are mostly no-ops since they're already protected by the outer one.
#[inline]
pub fn with<R>(f: impl FnOnce(CriticalSection) -> R) -> R {
    critical_section_1::with(|_| f(unsafe { CriticalSection::new() }))
}
