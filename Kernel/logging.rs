/*
 * Rust BareBones OS
 * - By John Hodge (Mutabah/thePowersGang) 
 *
 * logging.rs
 * - Debug output using rust's core::fmt system
 *
 * This code has been put into the public domain, there are no restrictions on
 * its use, and the author takes no liability.
 */
use core::sync::atomic::{AtomicUsize, Ordering};
use core::fmt;

/// A formatter object
pub struct Writer(usize);

/// A primitive lock for the logging output
///
/// This is not really a lock. Since there is no threading at the moment, all
/// it does is prevent writing when a collision would occur.
static LOGGING_LOCK: AtomicUsize = AtomicUsize::new(0);

impl Writer
{
	/// Obtain a logger for the specified module
	pub fn get(module: &str) -> Writer {
		// This "acquires" the lock (actually just disables output if paralel writes are attempted
                loop {
                    if (LOGGING_LOCK.compare_and_swap(0, 1, Ordering::SeqCst) == 0) {
                        break;
                    }
                }

		let mut ret = Writer(1);
		
		// Print the module name before returning (prefixes all messages)
		{
			use core::fmt::Write;
			let _ = write!(&mut ret, "[{}] ", module);
		}
		
		ret
	}
}

impl ::core::ops::Drop for Writer
{
	fn drop(&mut self)
	{
		// Write a terminating newline before releasing the lock
		{
			use core::fmt::Write;
			let _ = write!(self, "\n");
		}
		// On drop, "release" the lock
                LOGGING_LOCK.store(0, Ordering::SeqCst);
	}
}

impl fmt::Write for Writer
{
	fn write_str(&mut self, s: &str) -> fmt::Result
	{
		// If the lock is owned by this instance, then we can safely write to the output
                unsafe {
                        ::arch::debug::puts( s );
                }
		Ok( () )
	}
}

