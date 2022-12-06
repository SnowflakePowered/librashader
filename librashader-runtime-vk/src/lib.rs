#![feature(type_alias_impl_trait)]

mod hello_triangle;
mod filter_chain;
mod filter_pass;
mod error;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_vk() {
        hello_triangle::main();
    }
}
