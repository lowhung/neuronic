//! Bezier curve math and color interpolation functions.

use egui::{Color32, Pos2, Vec2};

/// Calculate a point on a quadratic Bezier curve.
///
/// # Arguments
///
/// * `p0` - Start point
/// * `p1` - Control point
/// * `p2` - End point
/// * `t` - Parameter in range [0, 1] where 0 = start, 1 = end
///
/// # Returns
///
/// The interpolated position on the curve.
pub fn quadratic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let t = t.clamp(0.0, 1.0);
    let mt = 1.0 - t;
    Pos2::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

/// Generate evenly-spaced points along a quadratic Bezier curve.
///
/// Used for drawing smooth curved edges between nodes.
///
/// # Arguments
///
/// * `p0` - Start point
/// * `p1` - Control point
/// * `p2` - End point
/// * `segments` - Number of line segments to generate
pub fn bezier_points(p0: Pos2, p1: Pos2, p2: Pos2, segments: usize) -> Vec<Pos2> {
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            quadratic_bezier(p0, p1, p2, t)
        })
        .collect()
}

/// Calculate the tangent direction at a point on a quadratic Bezier curve.
///
/// Used for orienting arrows and particles along edge curves.
///
/// # Arguments
///
/// * `p0` - Start point
/// * `p1` - Control point
/// * `p2` - End point
/// * `t` - Parameter in range [0, 1]
///
/// # Returns
///
/// A normalized direction vector tangent to the curve at parameter `t`.
pub fn bezier_tangent(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let mt = 1.0 - t;
    Vec2::new(
        2.0 * mt * (p1.x - p0.x) + 2.0 * t * (p2.x - p1.x),
        2.0 * mt * (p1.y - p0.y) + 2.0 * t * (p2.y - p1.y),
    )
    .normalized()
}

/// Linearly interpolate between two colors.
///
/// # Arguments
///
/// * `a` - Start color (at t=0)
/// * `b` - End color (at t=1)
/// * `t` - Interpolation factor, clamped to [0, 1]
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_bezier_start_point() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let result = quadratic_bezier(p0, p1, p2, 0.0);
        assert!((result.x - p0.x).abs() < 0.001);
        assert!((result.y - p0.y).abs() < 0.001);
    }

    #[test]
    fn test_quadratic_bezier_end_point() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let result = quadratic_bezier(p0, p1, p2, 1.0);
        assert!((result.x - p2.x).abs() < 0.001);
        assert!((result.y - p2.y).abs() < 0.001);
    }

    #[test]
    fn test_quadratic_bezier_midpoint() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let result = quadratic_bezier(p0, p1, p2, 0.5);
        assert!((result.x - 50.0).abs() < 0.001);
        assert!((result.y - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_quadratic_bezier_clamps_t() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let result_neg = quadratic_bezier(p0, p1, p2, -0.5);
        assert!((result_neg.x - p0.x).abs() < 0.001);

        let result_over = quadratic_bezier(p0, p1, p2, 1.5);
        assert!((result_over.x - p2.x).abs() < 0.001);
    }

    #[test]
    fn test_bezier_points_count() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let points = bezier_points(p0, p1, p2, 10);
        assert_eq!(points.len(), 11);
    }

    #[test]
    fn test_bezier_points_endpoints() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let points = bezier_points(p0, p1, p2, 10);
        assert!((points[0].x - p0.x).abs() < 0.001);
        assert!((points[0].y - p0.y).abs() < 0.001);
        assert!((points[10].x - p2.x).abs() < 0.001);
        assert!((points[10].y - p2.y).abs() < 0.001);
    }

    #[test]
    fn test_bezier_tangent_start() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 50.0);
        let p2 = Pos2::new(100.0, 0.0);

        let tangent = bezier_tangent(p0, p1, p2, 0.0);
        assert!((tangent.x - 0.707).abs() < 0.01);
        assert!((tangent.y - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_bezier_tangent_end() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 50.0);
        let p2 = Pos2::new(100.0, 0.0);

        let tangent = bezier_tangent(p0, p1, p2, 1.0);
        assert!((tangent.x - 0.707).abs() < 0.01);
        assert!((tangent.y - (-0.707)).abs() < 0.01);
    }

    #[test]
    fn test_bezier_tangent_is_normalized() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(100.0, 200.0);
        let p2 = Pos2::new(300.0, 50.0);

        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let tangent = bezier_tangent(p0, p1, p2, t);
            let length = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
            assert!(
                (length - 1.0).abs() < 0.001,
                "Tangent at t={} not normalized",
                t
            );
        }
    }

    #[test]
    fn test_lerp_color_start() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);

        let result = lerp_color(a, b, 0.0);
        assert_eq!(result.r(), 0);
        assert_eq!(result.g(), 0);
        assert_eq!(result.b(), 0);
    }

    #[test]
    fn test_lerp_color_end() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);

        let result = lerp_color(a, b, 1.0);
        assert_eq!(result.r(), 255);
        assert_eq!(result.g(), 255);
        assert_eq!(result.b(), 255);
    }

    #[test]
    fn test_lerp_color_midpoint() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(200, 100, 50);

        let result = lerp_color(a, b, 0.5);
        assert_eq!(result.r(), 100);
        assert_eq!(result.g(), 50);
        assert_eq!(result.b(), 25);
    }

    #[test]
    fn test_lerp_color_with_alpha() {
        let a = Color32::from_rgba_unmultiplied(100, 100, 100, 0);
        let b = Color32::from_rgba_unmultiplied(100, 100, 100, 200);

        let result = lerp_color(a, b, 0.5);
        assert_eq!(result.a(), 100);
    }

    #[test]
    fn test_lerp_color_clamps() {
        let a = Color32::from_rgb(50, 50, 50);
        let b = Color32::from_rgb(100, 100, 100);

        let result_neg = lerp_color(a, b, -0.5);
        assert_eq!(result_neg.r(), 50);

        let result_over = lerp_color(a, b, 1.5);
        assert_eq!(result_over.r(), 100);
    }
}
