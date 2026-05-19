pub fn clamp01(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let f = 1.0 - clamp01(t);
    1.0 - f * f * f
}

pub fn ease_in_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * t
}

pub fn smoothstep(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

/// Frame-rate-independent exponential decay factor. Returns the fraction of
/// the remaining distance to cover this frame, given a time-constant `tau`
/// (seconds to cover ~63% of the gap).
pub fn decay_factor(dt: f32, tau: f32) -> f32 {
    if tau <= 0.0 {
        1.0
    } else {
        1.0 - (-dt / tau).exp()
    }
}
