use crate::{
    common::{Method, RequestData},
    Result,
};

extern "C" {
    fn http_call_request(bytes: u32, bytes_len: u32) -> u64;
}

/// Calls http request, then returns body
pub fn call_request(
    url: String,
    headers: Vec<String>,
    queries: Vec<String>,
    method: Method,
) -> Result<Vec<u8>> {
    // Call ffi function
    let data = RequestData {
        url,
        headers,
        queries,
        method,
    };
    let bytes = rmp_serde::to_vec_named(&data)?;
    let offset = unsafe { http_call_request(bytes.as_ptr() as u32, bytes.len() as u32) };

    // Extract [len, data]
    if offset == 0 {
        Err("Expected data, got null ptr")?
    } else {
        let slice = unsafe {
            let len = *(offset as *const u32);
            let data = (offset + 4) as *const u8;
            std::slice::from_raw_parts(data, len as usize)
        };
        Ok(rmp_serde::from_slice(slice)?)
    }
}
