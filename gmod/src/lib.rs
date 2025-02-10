//! [Available Lua Functions](https://docs.rs/gmod/latest/gmod/lua/struct.State.html)

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::result_unit_err)]

#[cfg(not(all(
    any(target_os = "windows", target_os = "linux", target_os = "macos"),
    any(target_pointer_width = "32", target_pointer_width = "64")
)))]
compile_error!("Unsupported platform");

pub use gmod_macros::*;
pub use libloading;

/// Lua interface
pub mod lua;
pub use lua::task_queue::wait_lua_tick;
pub use lua::*;

/// Userdata types
pub mod userdata;

/// Net library helpers
pub mod net;

pub use ::defer::defer;

/// Returns whether this client is running the x86-64 branch
pub fn is_x86_64() -> bool {
    #[cfg(target_pointer_width = "64")]
    {
        // 64-bit can only be x86-64
        true
    }

    #[cfg(target_pointer_width = "32")]
    {
        use std::sync::LazyLock;
        static IS_X86_64: LazyLock<bool> = LazyLock::new(|| {
            {
                use std::path::PathBuf;

                #[cfg(target_os = "macos")]
                {
                    PathBuf::from("garrysmod/bin/lua_shared.dylib").is_file()
                }
                #[cfg(target_os = "windows")]
                {
                    PathBuf::from("srcds_win64.exe").is_file()
                }
                #[cfg(target_os = "linux")]
                {
                    // Check executable name
                    match std::env::current_exe()
                        .expect("Failed to get executable path")
                        .file_name()
                        .expect("Failed to get executable file name")
                        .to_string_lossy()
                        .as_ref()
                    {
                        #[cfg(target_os = "windows")]
                        "srcds.exe" => false,

                        #[cfg(target_os = "linux")]
                        "srcds_linux" => false,

                        #[cfg(target_os = "linux")]
                        "srcds" => true,

                        _ => {
                            // Check bin folder
                            #[cfg(target_os = "linux")]
                            {
                                PathBuf::from("bin/linux64").is_dir()
                            }
                            #[cfg(target_os = "windows")]
                            {
                                PathBuf::from("bin/win64").is_dir()
                            }
                        }
                    }
                }
            }
        });

        *IS_X86_64
    }
}

/// Opens & returns a shared library loaded by Garry's Mod using the raw path to the module.
///
/// # Example
/// ```no_run
/// // This would only work on Windows x86-64 branch in 64-bit mode
/// let (engine, engine_path): (gmod::libloading::Library, &'static str) = open_library_srv!("bin/win64/engine.dll").expect("Failed to open engine.dll!");
/// println!("Opened engine.dll from: {}", engine_path);
/// ```
#[macro_export]
macro_rules! open_library_raw {
	($($path:literal),+) => {
		match $crate::libloading::Library::new(concat!($($path),+)) {
			Ok(lib) => Ok((lib, concat!($($path),+))),
			Err(err) => Err((err, concat!($($path),+)))
		}
	}
}

/// Opens & returns a shared library loaded by Garry's Mod, in "server mode" (will prioritize _srv.so on Linux main branch)
///
/// Respects 32-bit/64-bit main/x86-64 branches and finds the correct library.
///
/// # Example
/// ```no_run
/// let (engine, engine_path): (gmod::libloading::Library, &'static str) = open_library_srv!("engine").expect("Failed to open engine.dll!");
/// println!("Opened engine.dll from: {}", engine_path);
/// ```
#[macro_export]
macro_rules! open_library_srv {
	($name:literal) => {{
		#[cfg(all(target_os = "windows", target_pointer_width = "64"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/win64/", $name, ".dll"),
				$crate::open_library_raw!($name)
			}
		}
		#[cfg(all(target_os = "windows", target_pointer_width = "32"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/", $name, ".dll"),
				$crate::open_library_raw!("garrysmod/bin/", $name, ".dll"),
				$crate::open_library_raw!($name)
			}
		}

		#[cfg(all(target_os = "linux", target_pointer_width = "64"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/linux64/", $name, ".so"),
				$crate::open_library_raw!("bin/linux64/lib", $name, ".so"),
				$crate::open_library_raw!($name)
			}
		}
		#[cfg(all(target_os = "linux", target_pointer_width = "32"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/linux32/", $name, ".so"),
				$crate::open_library_raw!("bin/linux32/lib", $name, ".so"),
				$crate::open_library_raw!("bin/", $name, "_srv.so"),
				$crate::open_library_raw!("bin/lib", $name, "_srv.so"),
				$crate::open_library_raw!("garrysmod/bin/", $name, "_srv.so"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, "_srv.so"),
				$crate::open_library_raw!("bin/", $name, ".so"),
				$crate::open_library_raw!("bin/lib", $name, ".so"),
				$crate::open_library_raw!("garrysmod/bin/", $name, ".so"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, ".so"),
				$crate::open_library_raw!($name)
			}
		}

		#[cfg(target_os = "macos")] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("GarrysMod_Signed.app/Contents/MacOS/", $name, ".dylib"),
				$crate::open_library_raw!("GarrysMod_Signed.app/Contents/MacOS/lib", $name, ".dylib"),
				$crate::open_library_raw!("bin/", $name, "_srv.dylib"),
				$crate::open_library_raw!("bin/lib", $name, "_srv.dylib"),
				$crate::open_library_raw!("garrysmod/bin/", $name, "_srv.dylib"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, "_srv.dylib"),
				$crate::open_library_raw!("bin/", $name, ".dylib"),
				$crate::open_library_raw!("bin/lib", $name, ".dylib"),
				$crate::open_library_raw!("garrysmod/bin/", $name, ".dylib"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, ".dylib"),
				$crate::open_library_raw!($name)
			}
		}
	}};
}

/// Opens & returns a shared library loaded by Garry's Mod. You are most likely looking for `open_library_srv!`, as this will prioritize non-_srv.so libraries on Linux main branch.
///
/// Respects 32-bit/64-bit main/x86-64 branches and finds the correct library.
///
/// # Example
/// ```no_run
/// let (engine, engine_path): (gmod::libloading::Library, &'static str) = open_library!("engine").expect("Failed to open engine.dll!");
/// println!("Opened engine.dll from: {}", engine_path);
/// ```
#[macro_export]
macro_rules! open_library {
	($name:literal) => {{
		#[cfg(all(target_os = "windows", target_pointer_width = "64"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/win64/", $name, ".dll"),
				$crate::open_library_raw!($name)
			}
		}
		#[cfg(all(target_os = "windows", target_pointer_width = "32"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/", $name, ".dll"),
				$crate::open_library_raw!("garrysmod/bin/", $name, ".dll"),
				$crate::open_library_raw!($name)
			}
		}

		#[cfg(all(target_os = "linux", target_pointer_width = "64"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/linux64/", $name, ".so"),
				$crate::open_library_raw!("bin/linux64/lib", $name, ".so"),
				$crate::open_library_raw!($name)
			}
		}
		#[cfg(all(target_os = "linux", target_pointer_width = "32"))] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("bin/linux32/", $name, ".so"),
				$crate::open_library_raw!("bin/linux32/lib", $name, ".so"),
				$crate::open_library_raw!("bin/", $name, ".so"),
				$crate::open_library_raw!("bin/lib", $name, ".so"),
				$crate::open_library_raw!("garrysmod/bin/", $name, ".so"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, ".so"),
				$crate::open_library_raw!("bin/", $name, "_srv.so"),
				$crate::open_library_raw!("bin/lib", $name, "_srv.so"),
				$crate::open_library_raw!("garrysmod/bin/", $name, "_srv.so"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, "_srv.so"),
				$crate::open_library_raw!($name)
			}
		}

		#[cfg(target_os = "macos")] {
			$crate::__private__gmod_rs__try_chained_open! {
				$crate::open_library_raw!("GarrysMod_Signed.app/Contents/MacOS/", $name, ".dylib"),
				$crate::open_library_raw!("GarrysMod_Signed.app/Contents/MacOS/lib", $name, ".dylib"),
				$crate::open_library_raw!("bin/", $name, ".dylib"),
				$crate::open_library_raw!("bin/lib", $name, ".dylib"),
				$crate::open_library_raw!("garrysmod/bin/", $name, ".dylib"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, ".dylib"),
				$crate::open_library_raw!("bin/", $name, "_srv.dylib"),
				$crate::open_library_raw!("bin/lib", $name, "_srv.dylib"),
				$crate::open_library_raw!("garrysmod/bin/", $name, "_srv.dylib"),
				$crate::open_library_raw!("garrysmod/bin/lib", $name, "_srv.dylib"),
				$crate::open_library_raw!($name)
			}
		}
	}};
}

#[derive(Default)]
#[doc(hidden)]
pub struct OpenGmodLibraryErrs(pub std::collections::HashMap<&'static str, libloading::Error>);
impl std::error::Error for OpenGmodLibraryErrs {}
impl std::fmt::Display for OpenGmodLibraryErrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for (path, err) in &self.0 {
            writeln!(f, "{} = {}", path, err)?;
        }
        writeln!(f)?;
        Ok(())
    }
}
impl std::fmt::Debug for OpenGmodLibraryErrs {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __private__gmod_rs__try_chained_open {
	{$($expr:expr),+} => {
		loop {
			let mut errors = $crate::OpenGmodLibraryErrs::default();
			$(
				match $expr {
					Ok(val) => break Ok(val),
					Err((err, path)) => { errors.0.insert(path, err); }
				}
			)+
			break Err(errors);
		}
	};
}

#[macro_export]
macro_rules! rstr {
    ($cstring:expr) => {{
        let cstring_ptr = $cstring;
        let cstr = unsafe { std::ffi::CStr::from_ptr(cstring_ptr) };
        cstr.to_str().expect("Couldn't unwrap CString")
    }};
}

#[macro_export]
macro_rules! lua_regs {
	() => {
        &[
            LuaReg {
                name: std::ptr::null(),
                func: None,
            }
        ]
    };
    (
        $(
            $name:literal => $func:expr
        ),* $(,)?
    ) => {
        &[
            $(
                LuaReg {
                    name: concat!($name, "\0").as_ptr() as *const i8,
                    func: Some($func),
                }
            ),*,
            LuaReg {
                name: std::ptr::null(),
                func: None,
            }
        ]
    };
}

pub fn cstring(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).expect("Failed to create CString")
}
