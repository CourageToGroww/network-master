/// Computes a network quality score from 0.0 (worst) to 100.0 (best).
///
/// Based on a weighted combination of latency, jitter, and packet loss.
/// The thresholds are tuned for general internet connectivity monitoring.
pub fn compute_quality_score(avg_rtt_ms: f64, jitter_ms: f64, loss_pct: f64) -> f64 {
    const W_LATENCY: f64 = 0.35;
    const W_JITTER: f64 = 0.25;
    const W_LOSS: f64 = 0.40;

    // Latency score: 100 at 0ms, 0 at 500ms+
    let latency_score = (1.0 - (avg_rtt_ms / 500.0).min(1.0)) * 100.0;

    // Jitter score: 100 at 0ms, 0 at 100ms+
    let jitter_score = (1.0 - (jitter_ms / 100.0).min(1.0)) * 100.0;

    // Loss score: 100 at 0%, 0 at 10%+
    let loss_score = (1.0 - (loss_pct / 10.0).min(1.0)) * 100.0;

    (W_LATENCY * latency_score + W_JITTER * jitter_score + W_LOSS * loss_score)
        .clamp(0.0, 100.0)
}

/// Returns a human-readable quality label based on the score.
pub fn quality_label(score: f64) -> &'static str {
    match score as u32 {
        90..=100 => "Excellent",
        75..=89 => "Good",
        50..=74 => "Fair",
        25..=49 => "Poor",
        _ => "Critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_connection() {
        let score = compute_quality_score(0.0, 0.0, 0.0);
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn terrible_connection() {
        let score = compute_quality_score(500.0, 100.0, 10.0);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn moderate_connection() {
        let score = compute_quality_score(50.0, 10.0, 1.0);
        assert!(score > 50.0 && score < 100.0);
    }

    #[test]
    fn clamps_above_thresholds() {
        let score = compute_quality_score(1000.0, 500.0, 50.0);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }
}
