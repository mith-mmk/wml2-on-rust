pub(crate) fn fdct_block(input: &[f32; 64]) -> [f32; 64] {
    let mut out = [0.0_f32; 64];
    let pi = std::f32::consts::PI;

    for v in 0..8 {
        for u in 0..8 {
            let mut sum = 0.0;
            for y in 0..8 {
                for x in 0..8 {
                    let sample = input[y * 8 + x];
                    let cos_x = (((2 * x + 1) as f32) * u as f32 * pi / 16.0).cos();
                    let cos_y = (((2 * y + 1) as f32) * v as f32 * pi / 16.0).cos();
                    sum += sample * cos_x * cos_y;
                }
            }
            let cu = if u == 0 { 1.0 / 2.0_f32.sqrt() } else { 1.0 };
            let cv = if v == 0 { 1.0 / 2.0_f32.sqrt() } else { 1.0 };
            out[v * 8 + u] = 0.25 * cu * cv * sum;
        }
    }

    out
}
