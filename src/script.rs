use js::{
    JSCLASS_GLOBAL_SLOT_COUNT,
    JSCLASS_IS_GLOBAL,
    JSCLASS_RESERVED_SLOTS_MASK
};
use js::jsapi::{
    CallArgs,
    CompartmentOptions,
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
    OnNewGlobalHookOption,
    Value
};
use js::jsval::UndefinedValue;
use js::rust::{Runtime, ToString};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr::null_mut;
use std::str::from_utf8;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::{JoinHandle, spawn};

pub enum ScriptMessage {
    Shutdown,
    EvaluateScript(String)
}

pub struct ScriptThread {
    sender: Sender<ScriptMessage>,
    handle: JoinHandle<()>
}

pub fn start() -> ScriptThread {
    let (sender, receiver) = mpsc::channel();
    let handle = spawn(move || {
        unsafe { JS_Init() };
        let rt = Runtime::new();
        let cx = rt.cx();
        rooted!(in(cx) let global = unsafe { JS_NewGlobalObject(cx, &CLASS, null_mut(), OnNewGlobalHookOption::FireOnNewGlobalHook, &CompartmentOptions::default()) });
        let _ac = JSAutoCompartment::new(cx, global.get());
        unsafe {
            JS_InitStandardClasses(cx, global.handle());
            JS_DefineFunction(cx, global.handle(), b"println\0".as_ptr() as *const c_char, Some(println), 1, 0);
        }
        while let Ok(message) = receiver.recv() {
            match message {
                ScriptMessage::Shutdown => {
                    break;
                }
                ScriptMessage::EvaluateScript(script) => {
                    rooted!(in(cx) let mut rval = UndefinedValue());
                    rt.evaluate_script(global.handle(), &script, &String::new(), 1, rval.handle_mut()).unwrap();
                }
            }
        }
    });
    ScriptThread { sender: sender, handle: handle }
}

pub fn shutdown(thread: ScriptThread) {
    thread.sender.send(ScriptMessage::Shutdown).unwrap();
    let _ = thread.handle.join();
}

pub fn evaluate_script(thread: &ScriptThread, script: &str) {
    thread.sender.send(ScriptMessage::EvaluateScript(String::from(script))).unwrap();
}

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

unsafe extern "C" fn println(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
    let args = CallArgs::from_vp(vp, argc);
    println!("{}", (0..argc).map(|i| value_to_string(cx, args.get(i))).collect::<Vec<String>>().join(" "));
    args.rval().set(UndefinedValue());
    true
}

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
