mod hello_triangle;

use ash::vk::Framebuffer;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_vk() {
        hello_triangle::main();
    }
}
