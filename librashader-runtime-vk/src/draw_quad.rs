#[rustfmt::skip]
static VBO_OFFSCREEN_MVP: &[f32; 16] = &[
    // Offscreen
    -1.0, -1.0, 0.0, 0.0,
    -1.0, 1.0, 0.0, 1.0,
    1.0, -1.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

#[rustfmt::skip]
static VBO_DEFAULT_FINAL_MVP: &[f32; 16] = &[
    // Final
    0.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 1.0,
    1.0, 0.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];
