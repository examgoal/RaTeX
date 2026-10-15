//! Android JNI bridge for RaTeX.
//!
//! Exposes `nativeParseAndLayout`, `nativeParseAndLayoutEx` and
//! `nativeGetLastError` as JNI methods on the `io.ratex.RaTeXEngine` Kotlin object.
//!
//! Compiled only when `target_os = "android"` (see lib.rs).

use jni::objects::{JClass, JFloatArray, JMethodID, JObject, JString};
use jni::signature::{Primitive, ReturnType};
use jni::sys::{jboolean, jint, jobject, jstring, jvalue, JNI_TRUE};
use jni::JNIEnv;

use crate::{ratex_get_last_error, ratex_parse_and_layout, RatexColor, RatexOptions};
use std::ffi::{c_void, CStr, CString};
use std::os::raw::{c_char, c_int};

/// JNI entry point for `RaTeXEngine.nativeParseAndLayout(latex: String, displayMode: Boolean, rgba: FloatArray): String?`
///
/// `displayMode = true`  → display (block) style  (`$$...$$`)
/// `displayMode = false` → inline (text) style     (`$...$`)
///
/// Returns a Java `String` on success, or `null` on error.
/// Call `nativeGetLastError()` to retrieve the error message.
#[no_mangle]
pub extern "system" fn Java_io_ratex_RaTeXEngine_nativeParseAndLayout(
    env: JNIEnv,
    _class: JClass,
    latex: JString,
    display_mode: jboolean,
    rgba: JFloatArray,
) -> jobject {
    parse_and_layout(env, latex, display_mode, rgba, 0, 0, None)
}

/// JNI entry point for
/// `RaTeXEngine.nativeParseAndLayoutEx(latex: String, displayMode: Boolean, rgba: FloatArray,
///  compat: Int, flags: Int, measurer: RaTeXTextMeasurer?): String?`
///
/// `compat`: 0 = KaTeX, 1 = MathJax. `flags`: `RATEX_FLAG_*` bits.
/// `measurer` (nullable) implements `fun measure(text: String, styleFlags: Int, out: FloatArray)`
/// and is called back on this thread, during this call, for every text-mode run.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_io_ratex_RaTeXEngine_nativeParseAndLayoutEx(
    env: JNIEnv,
    _class: JClass,
    latex: JString,
    display_mode: jboolean,
    rgba: JFloatArray,
    compat: jint,
    flags: jint,
    measurer: JObject,
) -> jobject {
    let measurer = (!measurer.is_null()).then_some(measurer);
    parse_and_layout(env, latex, display_mode, rgba, compat, flags, measurer)
}

/// State for [`measure_trampoline`]: raw JNI handles valid for the duration of
/// one `nativeParseAndLayoutEx` call (the measurer never outlives it).
struct JniMeasureCtx {
    env: *mut jni::sys::JNIEnv,
    measurer: jni::sys::jobject,
    method: JMethodID,
    out: jni::sys::jobject,
}

unsafe extern "C" fn measure_trampoline(
    ctx: *mut c_void,
    text: *const c_char,
    style_flags: u32,
    out: *mut f64,
) -> c_int {
    let ctx = unsafe { &*(ctx as *const JniMeasureCtx) };
    let Ok(mut env) = (unsafe { JNIEnv::from_raw(ctx.env) }) else {
        return 1;
    };
    let text = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    let Ok(jtext) = env.new_string(text.as_ref()) else {
        let _ = env.exception_clear();
        return 1;
    };
    let measurer = unsafe { JObject::from_raw(ctx.measurer) };
    let out_array = unsafe { JFloatArray::from_raw(ctx.out) };
    let args = [
        jvalue { l: jtext.as_raw() },
        jvalue {
            i: style_flags as jint,
        },
        jvalue {
            l: out_array.as_raw(),
        },
    ];
    let call = unsafe {
        env.call_method_unchecked(
            &measurer,
            ctx.method,
            ReturnType::Primitive(Primitive::Void),
            &args,
        )
    };
    let _ = env.delete_local_ref(jtext);
    if call.is_err() {
        // a Kotlin exception must not stay pending across the native frame
        let _ = env.exception_clear();
        return 1;
    }
    let mut values = [0.0f32; 3];
    if env
        .get_float_array_region(&out_array, 0, &mut values)
        .is_err()
    {
        let _ = env.exception_clear();
        return 1;
    }
    for (i, v) in values.iter().enumerate() {
        unsafe { *out.add(i) = f64::from(*v) };
    }
    // `measurer` / `out_array` wrap borrowed references owned by the caller
    std::mem::forget(measurer);
    std::mem::forget(out_array);
    0
}

fn parse_and_layout(
    mut env: JNIEnv,
    latex: JString,
    display_mode: jboolean,
    rgba: JFloatArray,
    compat: jint,
    flags: jint,
    measurer: Option<JObject>,
) -> jobject {
    let latex_str: String = match env.get_string(&latex) {
        Ok(s) => s.into(),
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("invalid latex string: {e}"),
            );
            return std::ptr::null_mut();
        }
    };

    let c_latex = match CString::new(latex_str) {
        Ok(cs) => cs,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("latex contains null byte: {e}"),
            );
            return std::ptr::null_mut();
        }
    };

    let color = {
        let len = match env.get_array_length(&rgba) {
            Ok(value) => value,
            Err(e) => {
                let _ = env.throw_new(
                    "java/lang/IllegalArgumentException",
                    format!("invalid color array: {e}"),
                );
                return std::ptr::null_mut();
            }
        };
        if len < 4 {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                "color array must contain 4 normalized RGBA floats",
            );
            return std::ptr::null_mut();
        }

        let mut values = [0.0f32; 4];
        if let Err(e) = env.get_float_array_region(&rgba, 0, &mut values) {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("failed to read color array: {e}"),
            );
            return std::ptr::null_mut();
        }
        RatexColor {
            r: values[0],
            g: values[1],
            b: values[2],
            a: values[3],
        }
    };

    // Resolve the measurer's method and scratch array once per call.
    let mut measure_ctx = None;
    if let Some(measurer) = measurer.as_ref() {
        let method = env
            .get_object_class(measurer)
            .and_then(|class| env.get_method_id(&class, "measure", "(Ljava/lang/String;I[F)V"));
        let out = env.new_float_array(3);
        match (method, out) {
            (Ok(method), Ok(out)) => {
                measure_ctx = Some(JniMeasureCtx {
                    env: env.get_raw(),
                    measurer: measurer.as_raw(),
                    method,
                    out: out.into_raw(),
                });
            }
            _ => {
                let _ = env.exception_clear();
                let _ = env.throw_new(
                    "java/lang/IllegalArgumentException",
                    "measurer must implement measure(String, Int, FloatArray)",
                );
                return std::ptr::null_mut();
            }
        }
    }

    let opts = RatexOptions {
        struct_size: std::mem::size_of::<RatexOptions>(),
        display_mode: if display_mode == JNI_TRUE { 1 } else { 0 },
        color: &color,
        compat,
        flags: flags as u32,
        measure_text: measure_ctx.as_ref().map(|_| {
            measure_trampoline
                as unsafe extern "C" fn(*mut c_void, *const c_char, u32, *mut f64) -> c_int
        }),
        measure_text_ctx: measure_ctx.as_mut().map_or(std::ptr::null_mut(), |c| {
            c as *mut JniMeasureCtx as *mut c_void
        }),
    };
    let result = unsafe { ratex_parse_and_layout(c_latex.as_ptr(), &opts) };
    if let Some(ctx) = measure_ctx {
        let _ = env.delete_local_ref(unsafe { JObject::from_raw(ctx.out) });
    }

    if result.error_code != 0 || result.data.is_null() {
        return std::ptr::null_mut();
    }

    let json = unsafe {
        std::ffi::CStr::from_ptr(result.data)
            .to_string_lossy()
            .into_owned()
    };
    unsafe { crate::ratex_free_display_list(result.data) };

    match env.new_string(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                format!("failed to create Java string: {e}"),
            );
            std::ptr::null_mut()
        }
    }
}

/// JNI entry point for `RaTeXEngine.nativeGetLastError(): String?`
///
/// Returns the last error message as a Java `String`, or `null` if no error.
#[no_mangle]
pub extern "system" fn Java_io_ratex_RaTeXEngine_nativeGetLastError(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let ptr = ratex_get_last_error();
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
    match env.new_string(msg.as_ref()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
