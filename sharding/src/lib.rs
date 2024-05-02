#[no_mangle]
pub extern "C" fn rust_function(number: i32) -> i32 {
    println!("Hello from Rust!");
    println!("Number: {}", number);
    return 15;
}
