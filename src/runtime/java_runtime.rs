use jni::errors::Error as JniError;
use jni::objects::{JByteArray, JObject, JString, JValue, JValueOwned};
use jni::signature::{Primitive, ReturnType};
use jni::sys::jvalue;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use super::{NativeSignature, NativeType, RuntimeError, RuntimeResult, Value};

static JAVA_RUNTIME: OnceLock<Arc<JavaRuntime>> = OnceLock::new();

pub struct JavaRuntime {
    vm: JavaVM,
}

impl JavaRuntime {
    pub fn initialize(jars: &[PathBuf]) -> RuntimeResult<Arc<Self>> {
        if let Some(handle) = JAVA_RUNTIME.get() {
            return Ok(handle.clone());
        }
        let class_path = Self::build_classpath(jars);
        let vm = Self::create_vm(&class_path)?;
        let runtime = Arc::new(JavaRuntime { vm });
        if JAVA_RUNTIME.set(runtime.clone()).is_err() {
            // Another thread initialized the VM first; reuse it.
            Ok(JAVA_RUNTIME.get().unwrap().clone())
        } else {
            Ok(runtime)
        }
    }

    pub fn instance() -> RuntimeResult<Arc<Self>> {
        JAVA_RUNTIME
            .get()
            .cloned()
            .ok_or_else(|| RuntimeError::new("Java VM not initialized"))
    }

    pub fn call_static_method(
        &self,
        class_name: &str,
        method_name: &str,
        signature: &NativeSignature,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|err| RuntimeError::new(format!("failed to attach JVM thread: {err}")))?;
        let class = env.find_class(class_name).map_err(|err| {
            RuntimeError::new(format!("failed to find class {class_name}: {err}"))
        })?;
        let descriptor = descriptor_for(signature);
        let method_id = env
            .get_static_method_id(&class, method_name, descriptor.as_str())
            .map_err(|err| RuntimeError::new(format!("method lookup {method_name}: {err}")))?;
        let owned_args = prepare_java_args(&env, signature, args)?;
        let raw_args: Vec<jvalue> = owned_args.iter().map(|arg| arg.as_jni()).collect();
        let return_type = return_type_for(signature);
        let result = unsafe {
            env.call_static_method_unchecked(class, method_id, return_type, raw_args.as_slice())
        }
        .map_err(|err| RuntimeError::new(format!("java call failed: {err}")))?;
        convert_java_return(&mut env, signature, result)
    }

    fn build_classpath(jars: &[PathBuf]) -> String {
        let separator = if cfg!(windows) { ";" } else { ":" };
        let mut entries = vec![];
        for jar in jars {
            entries.push(jar.display().to_string());
        }
        entries.sort();
        entries.dedup();
        env::var("CLASSPATH")
            .map(|existing| {
                if entries.is_empty() {
                    existing
                } else {
                    format!("{existing}{separator}{}", entries.join(separator))
                }
            })
            .unwrap_or_else(|_| entries.join(separator))
    }

    fn create_vm(class_path: &str) -> RuntimeResult<JavaVM> {
        let mut builder = InitArgsBuilder::new().version(JNIVersion::V8);
        let class_path_option = if class_path.is_empty() {
            None
        } else {
            Some(format!("-Djava.class.path={class_path}"))
        };
        if let Some(ref opt) = class_path_option {
            builder = builder.option(opt);
        }
        let args = builder.build().map_err(map_jni_error)?;
        JavaVM::new(args).map_err(map_jni_error)
    }
}

fn map_jni_error<E: std::error::Error>(err: E) -> RuntimeError {
    RuntimeError::new(format!("JNI error: {err}"))
}

fn descriptor_for(sig: &NativeSignature) -> String {
    let mut desc = String::from("(");
    for param in &sig.params {
        desc.push_str(native_type_descriptor(param));
    }
    desc.push(')');
    desc.push_str(return_descriptor(sig));
    desc
}

fn native_type_descriptor(kind: &NativeType) -> &'static str {
    match kind {
        NativeType::Str => "Ljava/lang/String;",
        NativeType::I32 => "I",
        NativeType::I64 => "J",
        NativeType::Bool => "Z",
        NativeType::Bytes => "[B",
    }
}

fn return_descriptor(sig: &NativeSignature) -> &'static str {
    match sig.return_type {
        Some(ref ty) => native_type_descriptor(ty),
        None => "V",
    }
}

fn return_type_for(sig: &NativeSignature) -> ReturnType {
    match sig.return_type {
        Some(NativeType::Str) => ReturnType::Object,
        Some(NativeType::Bytes) => ReturnType::Object,
        Some(NativeType::I32) => ReturnType::Primitive(Primitive::Int),
        Some(NativeType::I64) => ReturnType::Primitive(Primitive::Long),
        Some(NativeType::Bool) => ReturnType::Primitive(Primitive::Boolean),
        None => ReturnType::Primitive(Primitive::Void),
    }
}

fn prepare_java_args<'env>(
    env: &'env JNIEnv<'env>,
    sig: &NativeSignature,
    args: &[Value],
) -> RuntimeResult<Vec<JValueOwned<'env>>> {
    if args.len() != sig.params.len() {
        return Err(RuntimeError::new(format!(
            "java function expects {} arguments, got {}",
            sig.params.len(),
            args.len()
        )));
    }
    let mut prepared = Vec::with_capacity(args.len());
    for (value, ty) in args.iter().zip(&sig.params) {
        let arg = java_value_from(env, ty, value)?;
        prepared.push(arg);
    }
    Ok(prepared)
}

fn java_value_from<'env>(
    env: &'env JNIEnv<'env>,
    ty: &NativeType,
    value: &Value,
) -> RuntimeResult<JValueOwned<'env>> {
    match (ty, value) {
        (NativeType::Str, Value::String(s)) => {
            let jstr = env.new_string(s).map_err(map_jni_error)?;
            Ok(JValueOwned::Object(jstr.into()))
        }
        (NativeType::I32, Value::Int(i)) => Ok(JValueOwned::Int(*i as i32)),
        (NativeType::I64, Value::Int(i)) => Ok(JValueOwned::Long(*i)),
        (NativeType::Bool, Value::Bool(b)) => Ok(JValueOwned::Bool(if *b { 1 } else { 0 })),
        (NativeType::Bytes, Value::Vec(vec)) => {
            let mut bytes = Vec::new();
            for item in vec.borrow().iter() {
                if let Value::Int(i) = item {
                    bytes.push(*i as i8);
                } else {
                    return Err(RuntimeError::new(
                        "java byte array arguments must be vec<int>",
                    ));
                }
            }
            let array = env
                .new_byte_array(bytes.len() as i32)
                .map_err(map_jni_error)?;
            env.set_byte_array_region(&array, 0, &bytes)
                .map_err(map_jni_error)?;
            Ok(JValueOwned::Object(array.into()))
        }
        _ => Err(RuntimeError::new(format!(
            "java argument expected {}, got {}",
            native_type_descriptor(ty),
            value.type_name()
        ))),
    }
}

fn convert_java_return(
    env: &mut JNIEnv,
    sig: &NativeSignature,
    result: JValueOwned,
) -> RuntimeResult<Value> {
    match sig.return_type {
        Some(NativeType::Str) => {
            if let JValueOwned::Object(obj) = result {
                if obj.is_null() {
                    return Ok(Value::Null);
                }
                let s = JString::from(obj);
                let rust = env.get_string(&s).map_err(map_jni_error)?;
                Ok(Value::String(rust.into()))
            } else {
                Err(RuntimeError::new("java method returned wrong type"))
            }
        }
        Some(NativeType::Bytes) => {
            if let JValueOwned::Object(obj) = result {
                if obj.is_null() {
                    return Ok(Value::Null);
                }
                let arr = JByteArray::from(obj);
                let len = env.get_array_length(&arr).map_err(map_jni_error)?;
                let mut buffer = vec![0i8; len as usize];
                env.get_byte_array_region(&arr, 0, &mut buffer)
                    .map_err(map_jni_error)?;
                let vec_values = buffer
                    .into_iter()
                    .map(|b| Value::Int(b as i64))
                    .collect::<Vec<_>>();
                Ok(make_vec(vec_values))
            } else {
                Err(RuntimeError::new(
                    "java method returned wrong byte array type",
                ))
            }
        }
        Some(NativeType::I32) => {
            if let JValueOwned::Int(i) = result {
                Ok(Value::Int(i as i64))
            } else {
                Err(RuntimeError::new("java method returned wrong int32 type"))
            }
        }
        Some(NativeType::I64) => {
            if let JValueOwned::Long(i) = result {
                Ok(Value::Int(i))
            } else {
                Err(RuntimeError::new("java method returned wrong int64 type"))
            }
        }
        Some(NativeType::Bool) => {
            if let JValueOwned::Bool(b) = result {
                Ok(Value::Bool(b != 0))
            } else {
                Err(RuntimeError::new("java method returned wrong bool type"))
            }
        }
        None => Ok(Value::Null),
    }
}

fn make_vec(items: Vec<Value>) -> Value {
    Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(super::VecValue {
        elem_type: None,
        items,
    })))
}
