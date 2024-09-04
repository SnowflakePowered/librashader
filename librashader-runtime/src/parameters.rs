use arc_swap::ArcSwap;
use librashader_common::map::{FastHashMap, ShortString};
use librashader_presets::ParameterConfig;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Trait for filter chains that allow runtime reflection of shader parameters.
pub trait FilterChainParameters {
    /// Get the runtime parameters for this filter chain.
    fn parameters(&self) -> &RuntimeParameters;
}

/// Runtime reflection of shader parameters for filter chains.
///
/// All operations on runtime parameters are atomic and can be done on
/// any thread.
pub struct RuntimeParameters {
    passes_enabled: AtomicUsize,
    pub(crate) parameters: ArcSwap<FastHashMap<ShortString, f32>>,
}

impl RuntimeParameters {
    /// Create a new instance of runtime parameters from a `Vec` of
    /// shader parameters from a [`ShaderPreset`](librashader_presets::ShaderPreset).
    pub fn new(passes_enabled: usize, parameters: Vec<ParameterConfig>) -> Self {
        RuntimeParameters {
            passes_enabled: AtomicUsize::new(passes_enabled),
            parameters: ArcSwap::new(Arc::new(
                parameters
                    .into_iter()
                    .map(|param| (param.name, param.value))
                    .collect(),
            )),
        }
    }

    /// Get the value of a runtime parameter
    pub fn get_parameter(&self, name: &str) -> Option<f32> {
        self.parameters.load().get::<str>(name.as_ref()).copied()
    }

    /// Set a runtime parameter.
    ///
    /// This is a relatively slow operation as it will be synchronized across threads.
    pub fn set_parameter(&self, name: &str, new_value: f32) -> Option<f32> {
        let mut updated_map = FastHashMap::clone(&self.parameters.load());

        if let Some(value) = updated_map.get_mut::<str>(name.as_ref()) {
            let old = *value;
            *value = new_value;

            self.parameters.store(Arc::new(updated_map));

            Some(old)
        } else {
            None
        }
    }

    /// Get a reference to the runtime parameters.
    pub fn parameters(&self) -> Arc<FastHashMap<ShortString, f32>> {
        self.parameters.load_full()
    }

    /// Get the number of passes enabled.
    ///
    /// If set from [`RuntimeParameters::set_passes_enabled`] from a different thread,
    /// it is not guaranteed to be immediately visible.
    #[inline(always)]
    pub fn passes_enabled(&self) -> usize {
        self.passes_enabled.load(Ordering::Relaxed)
    }

    /// Set the number of passes enabled.
    ///
    /// This is an atomic operation and is thread-safe.
    #[inline(always)]
    pub fn set_passes_enabled(&self, count: usize) {
        self.passes_enabled.store(count, Ordering::Relaxed);
    }
}

#[macro_export]
macro_rules! impl_filter_chain_parameters {
    ($ty:ty) => {
        impl ::librashader_runtime::parameters::FilterChainParameters for $ty {
            fn parameters(&self) -> &::librashader_runtime::parameters::RuntimeParameters {
                &self.common.config
            }
        }
    };
}
