/// Cache a pipeline state object.
///
/// Keys are not used to create the object and are only used to uniquely identify the pipeline state.
///
/// - `restore_pipeline` tries to restore the pipeline with either a cached binary pipeline state
///    cache, or create a new pipeline if no cached value is available.
/// - `fetch_pipeline_state` fetches the new pipeline state cache after the pipeline was created.
pub fn cache_pipeline<E, T, R, const KEY_SIZE: usize>(
    index: &str,
    keys: &[&dyn CacheKey; KEY_SIZE],
    restore_pipeline: impl Fn(Option<Vec<u8>>) -> Result<R, E>,
    fetch_pipeline_state: impl FnOnce(&R) -> Result<T, E>,
    bypass_cache: bool,
) -> Result<R, E>
    where
        T: Cacheable,
{
    return Ok(restore_pipeline(None)?);
}

/// Cache a shader object (usually bytecode) created by the keyed objects.
///
/// - `factory` is the function that compiles the values passed as keys to a shader object.
/// - `load` tries to load a compiled shader object to a driver-specialized result.
pub fn cache_shader_object<E, T, R, H, const KEY_SIZE: usize>(
    index: &str,
    keys: &[H; KEY_SIZE],
    factory: impl FnOnce(&[H; KEY_SIZE]) -> Result<T, E>,
    load: impl Fn(T) -> Result<R, E>,
    bypass_cache: bool,
) -> Result<R, E>
    where
        H: CacheKey,
        T: Cacheable,
{
    return Ok(load(factory(keys)?)?);
}