use anyhow::{Result, bail};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Serialize, de::DeserializeOwned};
use std::{cell::RefCell, collections::HashMap, io::Cursor};
use wasmer::{
    Function, FunctionEnv, Instance, Memory, MemoryView, Module, Store, Value, ValueType, WasmSlice,
};
use wasmer_cache::{Cache, FileSystemCache, Hash};
use wasmer_wasi::WasiState;

pub mod ffi;

pub fn read<T: ValueType>(view: &MemoryView<'_>, offset: u64, length: u64) -> Result<Vec<T>> {
    Ok(WasmSlice::new(view, offset, length)?.read_to_vec()?)
}

pub struct Wasm {
    instance: Instance,
    store: RefCell<Store>,
}

impl Wasm {
    pub fn new(bytes: &[u8], env: HashMap<String, String>) -> Result<Self> {
        // Load module using cache
        let mut store = Store::default();
        let key = Hash::generate(bytes);
        let mut cache = FileSystemCache::new("cache")?;
        let module = match unsafe { cache.load(&store, key) } {
            Ok(module) => module,
            Err(_) => {
                let module = Module::new(&store, bytes)?;
                cache.store(key, &module)?;
                module
            }
        };

        // Initialize wasi
        let wasi_env = WasiState::new("space").envs(env).finalize(&mut store)?;
        let mut import_object = wasi_env.import_object(&mut store, &module)?;

        // Add environment
        let function_env = FunctionEnv::new(&mut store, ffi::SpaceEnv::default());
        import_object.define(
            "env",
            "http_call_request",
            Function::new_typed_with_env(&mut store, &function_env, ffi::http_call_request),
        );

        // Create instance
        let instance = Instance::new(&mut store, &module, &import_object)?;
        let memory = instance.exports.get_memory("memory")?;

        // Give reference to memory
        wasi_env.data_mut(&mut store).set_memory(memory.clone());
        function_env.as_mut(&mut store).set_memory(memory.clone());

        Ok(Self {
            instance,
            store: RefCell::new(store),
        })
    }

    fn call(&self, name: &str, values: &[Value]) -> Result<Box<[Value]>> {
        let method = self.instance.exports.get_function(name)?;
        Ok(method.call(&mut *self.store.borrow_mut(), values)?)
    }

    fn memory(&self) -> Result<&Memory> {
        Ok(self.instance.exports.get::<Memory>("memory")?)
    }

    fn view(&self) -> Result<MemoryView> {
        let store = self.store.borrow_mut();
        Ok(self.memory()?.view(&*store))
    }

    fn memory_grow(&self, size: usize) -> Result<()> {
        let memory = self.memory()?;
        let pages = (size / wasmer::WASM_PAGE_SIZE) + 1;
        memory.grow(&mut *self.store.borrow_mut(), pages as u32)?;
        Ok(())
    }

    pub fn run<T: Serialize, U: DeserializeOwned>(&self, name: &str, input: &T) -> Result<U> {
        // Serialize data
        let serialized = rmp_serde::to_vec(input)?;
        let input_len = (serialized.len() as u32).to_le_bytes();
        let input_bytes = [&input_len[..], &serialized].concat();

        // Write to memory
        let heap_start = match self
            .instance
            .exports
            .get::<wasmer::Global>("__heap_base")
            .map(|it| it.get(&mut *self.store.borrow_mut()))
        {
            Ok(Value::I32(heap_start)) => heap_start,
            _ => 0x110000,
        };
        self.memory_grow(input_bytes.len())?;
        self.view()?.write(heap_start as u64, &input_bytes)?;

        // Call module and pass pointer
        let values = self.call(name, &[Value::I32(heap_start)])?;

        // Deserialize data from pointer
        match &values[..] {
            [Value::I32(pointer)] => {
                let output_len = {
                    let bytes = read::<u8>(&self.view()?, *pointer as u64, 4)?;
                    bytes.as_slice().read_u32::<LittleEndian>()?
                };
                let output_ptr = {
                    let bytes = read::<u8>(&self.view()?, *pointer as u64 + 4, 4)?;
                    bytes.as_slice().read_u32::<LittleEndian>()?
                };
                let output_bytes = read::<u8>(&self.view()?, output_ptr as u64, output_len as u64)?;
                let output_buffer = Cursor::new(output_bytes);
                Ok(rmp_serde::from_read(output_buffer)?)
            }
            _ => bail!("Expected pointer to serialized data, got {values:#?}"),
        }
    }
}

// #[cfg(test)]
// mod tests;
