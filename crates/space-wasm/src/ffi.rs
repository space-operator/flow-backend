use crate::read;
use serde::Serialize;
use space_lib::{
    common::{Method, RequestData},
    Result,
};
use wasmer::{AsStoreRef, FunctionEnvMut, Memory, MemoryView};

// Environment
#[derive(Default)]
pub struct SpaceEnv {
    memory: Option<Memory>,
}

impl SpaceEnv {
    pub fn set_memory(&mut self, memory: Memory) {
        self.memory = Some(memory);
    }

    fn get_memory(&self) -> &Memory {
        self.memory.as_ref().unwrap()
    }

    fn view<'a>(&'a self, store: &'a impl AsStoreRef) -> MemoryView<'a> {
        self.get_memory().view(store)
    }
}

// Utility functions
fn create_request(request_data: RequestData) -> ureq::Request {
    let mut request = match request_data.method {
        Method::GET => ureq::get(&request_data.url),
        Method::POST => ureq::post(&request_data.url),
        Method::DELETE => ureq::delete(&request_data.url),
        Method::HEAD => ureq::head(&request_data.url),
        Method::PATCH => ureq::patch(&request_data.url),
        Method::PUT => ureq::put(&request_data.url),
    };
    for chunk in request_data.headers.chunks(2) {
        let (header, value) = (&chunk[0], &chunk[1]);
        request = request.set(header, value);
    }
    for chunk in request_data.queries.chunks(2) {
        let (param, value) = (&chunk[0], &chunk[1]);
        request = request.query(param, value);
    }
    request
}

fn write_serialized<T: Serialize>(mut ctx: FunctionEnvMut<SpaceEnv>, value: T) -> Result<u64> {
    // Read response into string
    let data = rmp_serde::to_vec_named(&value)?;

    // Find offset to write data, possibly grow memory
    let memory_size = ctx.data().view(&ctx).data_size();
    let offset = memory_size;
    let total = data.len() as u64 + 4;
    let delta = (offset + total - memory_size) / wasmer::WASM_PAGE_SIZE as u64 + 1;
    let memory = ctx.data().get_memory().clone();
    memory.grow(&mut ctx, delta as u32)?;

    // Write bytes as [len, data]
    let view = ctx.data().view(&ctx);
    view.write(offset, &u32::to_le_bytes(data.len() as u32))?;
    view.write(offset + 4, &data)?;

    // Return pointer to SpaceSlice
    Ok(offset)
}

// Host functions
pub fn http_call_request(ctx: FunctionEnvMut<SpaceEnv>, bytes: u32, bytes_len: u32) -> u64 {
    let stub = |ctx: FunctionEnvMut<SpaceEnv>, bytes, bytes_len| -> Result<u64> {
        // Setup environment
        let env = ctx.data();
        let view = env.view(&ctx);
        let raw_bytes = read::<u8>(&view, bytes as u64, bytes_len as u64)?;
        let request_data = rmp_serde::from_slice::<RequestData>(&raw_bytes)?;
        let request = create_request(request_data);
        let mut response = Vec::new();
        let mut reader = request.call()?.into_reader();
        reader.read_to_end(&mut response)?;
        write_serialized(ctx, response)
    };
    stub(ctx, bytes, bytes_len).unwrap_or(0)
}
