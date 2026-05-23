pub mod harden_normalize;
pub mod link_input_outputs;
pub mod lower_loop_sample_lod;
pub mod lower_samplers;
pub mod split_io_arrays;

// Load SPIR-V as an rspirv module
pub(crate) fn load_module(words: &[u32]) -> rspirv::dr::Module {
    let mut loader = rspirv::dr::Loader::new();
    rspirv::binary::parse_words(words, &mut loader).unwrap();
    let module = loader.module();
    module
}
