#![feature(type_alias_impl_trait)]
#![feature(let_chains)]

mod hello_triangle;
mod filter_chain;
mod filter_pass;
mod error;
mod util;
mod framebuffer;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_vk() {
        hello_triangle::main();
    }
}
