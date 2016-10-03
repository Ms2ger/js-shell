extern crate getopts;
#[macro_use]
extern crate js;

use getopts::Options;
use js::{
    JSCLASS_GLOBAL_SLOT_COUNT,
    JSCLASS_IS_GLOBAL,
    JSCLASS_RESERVED_SLOTS_MASK
};
use js::jsapi::{
    CallArgs,
    CompartmentOptions,
    HandleObject,
    HandleValue,
    JSAutoCompartment,
    JSCLASS_RESERVED_SLOTS_SHIFT,
    JSClass,
    JSClassOps,
    JSContext,
    JS_DefineFunction,
    JS_EncodeStringToUTF8,
    JS_GlobalObjectTraceHook,
    JS_Init,
    JS_InitStandardClasses,
    JS_NewGlobalObject,
    JS_free,
    MutableHandleValue,
    OnNewGlobalHookOption,
    Value
};
use js::jsval::UndefinedValue;
use js::rust::{Runtime, ToString};
use std::env::args;
use std::ffi::CStr;
use std::fs::File;
use std::io::Read;
use std::os::raw::{c_char, c_void};
use std::ptr::null_mut;
use std::str::from_utf8;

static CLASS_OPS: JSClassOps = JSClassOps {
    addProperty: None,
    delProperty: None,
    getProperty: None,
    setProperty: None,
    enumerate: None,
    resolve: None,
    mayResolve: None,
    finalize: None,
    call: None,
    hasInstance: None,
    construct: None,
    trace: Some(JS_GlobalObjectTraceHook)
};

static CLASS: JSClass = JSClass {
    name: b"global" as *const u8 as *const c_char,
    flags: JSCLASS_IS_GLOBAL | (JSCLASS_GLOBAL_SLOT_COUNT & JSCLASS_RESERVED_SLOTS_MASK) << JSCLASS_RESERVED_SLOTS_SHIFT,
    cOps: &CLASS_OPS,
    reserved: [0 as *mut _; 3]
};

fn value_to_string(cx: *mut JSContext, v: HandleValue) -> String {
    rooted!(in(cx) let str = unsafe {
        let str = ToString(cx, v);
        assert!(!str.is_null(), "Error converting value to string.");
        str
    });
    let slice = unsafe {
        let bytes = JS_EncodeStringToUTF8(cx, str.handle());
        assert!(!str.is_null(), "Error encoding string to UTF8.");
        CStr::from_ptr(bytes)
    };
    let result = String::from(from_utf8(slice.to_bytes()).unwrap());
    unsafe { JS_free(cx, slice.as_ptr() as *mut c_void) };
    result
}

unsafe extern "C" fn println(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
    let args = CallArgs::from_vp(vp, argc);
    println!("{}", (0..argc).map(|i| value_to_string(cx, args.get(i))).collect::<Vec<String>>().join(" "));
    args.rval().set(UndefinedValue());
    true
}

fn run_script(rt: &Runtime, global: HandleObject, filename: &str, rval: MutableHandleValue) {
    let mut source = String::new();
    {
        let mut file = match File::open(&filename) {
            Ok(file) => file,
            Err(_) => panic!("Error opening file.")
        };
        if let Err(_) = file.read_to_string(&mut source) {
            panic!("Error reading file.");
        }
    }
    if let Err(_) = rt.evaluate_script(global, &source, filename, 1, rval) {
        panic!("Error evaluating script.");
    }
}

fn main() {
    unsafe { JS_Init() };
    let rt = Runtime::new();
    let cx = rt.cx();
    rooted!(in(cx) let global = unsafe { JS_NewGlobalObject(cx, &CLASS, null_mut(), OnNewGlobalHookOption::FireOnNewGlobalHook, &CompartmentOptions::default()) });
    let _ac = JSAutoCompartment::new(cx, global.get());
    unsafe {
        JS_InitStandardClasses(cx, global.handle());
        JS_DefineFunction(cx, global.handle(), b"println\0".as_ptr() as *const c_char, Some(println), 1, 0);
    }
    let opts = Options::new();
    let args: Vec<String> = args().collect();
    let free = match opts.parse(&args[1..]) {
        Ok(matches) => matches,
        Err(_) => {
            panic!("Error parsing options.");
        }
    }.free;
    if !free.is_empty() {
        rooted!(in(cx) let mut rval = UndefinedValue());
        run_script(&rt, global.handle(), &free[0], rval.handle_mut());
        println!("{}", value_to_string(cx, rval.handle()));
    }
}
