macro_rules! ffi_body {
    ($body:block) => {
        {
            let result: Result<(), $crate::error::LibrashaderError> = try {
                $body
            };

            let Err(e) = result else {
                return $crate::error::LibrashaderError::ok()
            };
            e.export()
        }
    };
    (|$($ref_capture:ident),*|; mut |$($mut_capture:ident),*| $body:block) => {
        {
            $($crate::error::assert_non_null!($ref_capture);)*
            $(let $ref_capture = unsafe { &*$ref_capture };)*
            $($crate::error::assert_non_null!($mut_capture);)*
            $(let $mut_capture = unsafe { &mut *$mut_capture };)*
            let result: Result<(), $crate::error::LibrashaderError> = try {
                $body
            };

            let Err(e) = result else {
                return $crate::error::LibrashaderError::ok()
            };
            e.export()
        }
    };
    (mut |$($mut_capture:ident),*| $body:block) => {
        {
            $($crate::error::assert_non_null!($mut_capture);)*
            $(let $mut_capture = unsafe { &mut *$mut_capture };)*
            let result: Result<(), $crate::error::LibrashaderError> = try {
                $body
            };

            let Err(e) = result else {
                return $crate::error::LibrashaderError::ok()
            };
            e.export()
        }
    };
    (|$($ref_capture:ident),*| $body:block) => {
        {
            $($crate::error::assert_non_null!($ref_capture);)*
            $(let $ref_capture = unsafe { &*$ref_capture };)*
            let result: Result<(), $crate::error::LibrashaderError> = try {
                $body
            };

            let Err(e) = result else {
                return $crate::error::LibrashaderError::ok()
            };
            e.export()
        }
    }
}

macro_rules! extern_fn {
    ($(#[$($attrss:tt)*])* fn $func_name:ident ($($arg_name:ident : $arg_ty:ty),*) $body:block) => {
        paste::paste! {
            pub type [<PFN_ $func_name>] = unsafe extern "C" fn($($arg_name: $arg_ty,)*) -> $crate::ctypes::libra_error_t;
        }

        #[no_mangle]
        $(#[$($attrss)*])*
        fn $func_name($($arg_name: $arg_ty,)*) -> $crate::ctypes::libra_error_t {
            $crate::ffi::ffi_body!($body)
        }
    }
}

pub(crate) use extern_fn;
pub(crate) use ffi_body;
