use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
struct Input {
    value: usize,
    name: String,
}

#[derive(Serialize)]
struct Output {
    value: usize,
    name: String,
}

#[repr(C)]
struct SpaceSlice {
    len: usize,
    ptr: *mut u8,
}

#[no_mangle]
fn main(ptr: usize) -> Box<SpaceSlice> {
    // Deserialize input
    let bytes = unsafe {
        let len = *(ptr as *const usize);
        let data = (ptr + 4) as *mut u8;
        std::slice::from_raw_parts(data, len)
    };
    let input = rmp_serde::from_slice::<Input>(bytes).unwrap();

    // Actual code
    fn main_stub(input: Input) -> Output {
        Output {
            value: input.value * 2,
            name: input.name.chars().rev().collect(),
        }
    }
    let output = main_stub(input);
    
    // Serialize output
    let bytes = rmp_serde::to_vec_named(&output).unwrap().leak();
    Box::new(SpaceSlice {
        len: bytes.len(),
        ptr: bytes.as_mut_ptr(),
    })
}
