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

// The two flags together, for the evidence opens that want both. Written as
// one literal per target rather than `O_NOFOLLOW | O_NONBLOCK`: the bits are
// disjoint on every target here, so `|` and `^` produce the same value and the
// `^` mutant of that expression is equivalent -- no test can observe it.
// D-0192's rule for an equivalent mutant is to remove the expression, not to
// skip it. `crates/store/tests/open_flags.rs` pins each literal to the union
// of the two constants above, so this is not a second authority for either bit.

/// `O_NOFOLLOW | O_NONBLOCK` on `x86_64` Linux: `0x2_0000 | 0x800`.
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_arch = "x86_64"
))]
pub const O_NOFOLLOW_NONBLOCK: i32 = 0x0002_0800;

/// `O_NOFOLLOW | O_NONBLOCK` on `aarch64` Linux: `0x8000 | 0x800`.
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_arch = "aarch64"
))]
pub const O_NOFOLLOW_NONBLOCK: i32 = 0x8800;

/// `O_NOFOLLOW | O_NONBLOCK` on macOS: `0x100 | 0x4`.
#[cfg(target_os = "macos")]
pub const O_NOFOLLOW_NONBLOCK: i32 = 0x0104;
