/// Platform-specific buffer type aliases for unified API
/// Automatically selects the correct implementation per platform

#[cfg(target_os = "linux")]
pub use crate::ipc::shm_buffer_futex::FutexSharedMemoryBuffer as PlatformBuffer;

#[cfg(target_os = "macos")]
pub use crate::ipc::shm_buffer_macos::MacOsSharedMemoryBuffer as PlatformBuffer;

#[cfg(target_os = "windows")]
pub use crate::ipc::shm_buffer_windows::WindowsSharedMemoryBuffer as PlatformBuffer;

// Fallback for other Unix platforms
#[cfg(all(unix, not(target_os = "linux"), not(target_os = "macos")))]
pub use crate::ipc::shm_buffer_volatile::VolatileSharedMemoryBuffer as PlatformBuffer;

/// Platform-specific doorbell type aliases
#[cfg(target_os = "linux")]
pub use crate::ipc::eventfd_doorbell::EventFdDoorbell as PlatformDoorbell;

#[cfg(target_os = "macos")]
pub use crate::ipc::kqueue_doorbell::KqueueDoorbell as PlatformDoorbell;

#[cfg(target_os = "windows")]
pub use crate::ipc::windows_event::WindowsEvent as PlatformDoorbell;

#[cfg(all(unix, not(target_os = "linux"), not(target_os = "macos")))]
pub use crate::ipc::eventfd_doorbell::EventFdDoorbell as PlatformDoorbell;
