use afml_sdk::afml_export;

#[afml_export(signature = "fn greet(str) -> str")]
pub extern "C" fn greet(name: *const std::os::raw::c_char) -> *const std::os::raw::c_char {
    name
}

#[afml_export(signature = "fn add(i32, i32) -> i32")]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
