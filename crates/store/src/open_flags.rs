//! The one place the raw no-follow and non-blocking open flags are written.
//!
//! The workspace takes no `libc` dependency, so these are written out from the
//! platform headers. They are per *architecture*, not per OS: `O_NOFOLLOW` is
//! `0x20000` on `x86_64` Linux, but on `aarch64` Linux that bit is `O_LARGEFILE`
//! and `O_NOFOLLOW` is `0x8000`. A Linux-wide `0x20000` therefore opened
//! through a final symlink on `aarch64` (D-0980). glibc and musl share these
//! values because they come from the kernel UAPI, not the C library.
//!
//! Defined only for the targets verified here. Any other target has no
//! constant, so a caller without its own refusal branch fails to compile
//! rather than inheriting a wrong bit; `cli` adds an explicit
//! `compile_error!` for Linux on any other architecture.
//! `crates/store/tests/open_flags.rs` refuses the `x86_64` literal anywhere in
//! `crates/` outside this file.

/// `x86_64` Linux UAPI `include/uapi/asm-generic/fcntl.h`: `O_NOFOLLOW 00400000`.
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_arch = "x86_64"
))]
pub const O_NOFOLLOW: i32 = 0x0002_0000;

/// `aarch64` Linux UAPI `arch/arm64/include/uapi/asm/fcntl.h`: `O_NOFOLLOW 0100000`.
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_arch = "aarch64"
))]
pub const O_NOFOLLOW: i32 = 0x8000;

/// macOS SDK `sys/fcntl.h`: `O_NOFOLLOW 0x00000100`.
#[cfg(target_os = "macos")]
pub const O_NOFOLLOW: i32 = 0x0100;

/// Linux UAPI `asm-generic/fcntl.h`: `O_NONBLOCK 00004000`, which neither
/// `x86_64` nor arm64 overrides.
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub const O_NONBLOCK: i32 = 0x0800;

/// macOS SDK `sys/fcntl.h`: `O_NONBLOCK 0x00000004`.
#[cfg(target_os = "macos")]
pub const O_NONBLOCK: i32 = 0x0004;
