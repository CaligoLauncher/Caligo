//! Original isometric illustration for the UI example, not a logo or installed-build data.
//! Geometry is prepared once; no image decoding, filesystem, timers or 3D runtime.
use std::sync::OnceLock;
use gpui::{IntoElement, PathBuilder, canvas, point, prelude::*, px, rgba};

const N: usize = 22;
type Point = (f32, f32);

#[derive(Debug, PartialEq)]
struct Face {
    vertices: Vec<Point>,
    color: u32,
}

fn heightmap() -> [[u8; N]; N] {
    let mut heights = [[0; N]; N];
    for (i, row) in heights.iter_mut().enumerate() {
        for (j, height) in row.iter_mut().enumerate() {
            let u = (i as f32 - 10.5) / 10.9;
            let v = (j as f32 - 10.5) / 10.6;
            let edge = 1. + 0.07 * (i as f32 * 0.9).sin() + 0.06 * (j as f32 * 0.75).cos();
            if u * u + v * v > edge { continue; }
            let peak = 7.4 * (-((u + 0.3).powi(2) / 0.16 + (v + 0.17).powi(2) / 0.24)).exp()
                + 5.5 * (-((u - 0.42).powi(2) / 0.14 + (v + 0.13).powi(2) / 0.12)).exp();
            let ripple = 0.5 * (i as f32 * 0.8).sin() * (j as f32 * 0.65).cos();
            *height = if river(i, j) { 1 } else { (1.3 + peak + ripple).round().max(1.) as u8 };
        }
    }
    heights
}

fn river(i: usize, j: usize) -> bool {
    i > 4 && (j as f32 - (12. + 2. * (i as f32 * 0.25).sin())).abs() < 1.1
}

fn project(i: f32, j: f32, z: f32) -> Point {
    (198. + (i - j) * 9., 44. + (i + j) * 4.5 - z * 6.5)
}

fn generate() -> Vec<Face> {
    let heights = heightmap();
    let mut faces = Vec::with_capacity(N * N * 3);
    for depth in 0..(2 * N - 1) {
        for i in 0..N {
            if depth < i || depth - i >= N { continue; }
            let j = depth - i;
            let h = heights[i][j];
            if h == 0 { continue; }
            let hi = if i + 1 < N { heights[i + 1][j] } else { 0 };
            let hj = if j + 1 < N { heights[i][j + 1] } else { 0 };
            let x = i as f32;
            let y = j as f32;
            let z = h as f32;
            let alpha = ((0.58 + 0.35 * depth as f32 / 42.) * 255.) as u32;
            let mut add = |vertices: &[(f32, f32, f32)], color: u32, opacity: u32| {
                faces.push(Face {
                    vertices: vertices.iter().map(|&(a, b, c)| project(a, b, c)).collect(),
                    color: (color << 8) | opacity,
                });
            };
            if hi < h {
                add(&[(x+1.,y,z),(x+1.,y+1.,z),(x+1.,y+1.,hi as f32),(x+1.,y,hi as f32)],
                    0x809eac, alpha * 4 / 5);
            }
            if hj < h {
                add(&[(x,y+1.,z),(x+1.,y+1.,z),(x+1.,y+1.,hj as f32),(x,y+1.,hj as f32)],
                    0x648797, alpha * 4 / 5);
            }
            let top = if river(i,j) { 0xb7d6df } else if h > 5 { 0xedf5f6 }
                else if h > 2 { 0xc9dbdf } else { 0xbbd0d5 };
            add(&[(x,y,z),(x+1.,y,z),(x+1.,y+1.,z),(x,y+1.,z)], top, alpha);
            if !river(i,j) && h < 4 && i > 2 && i < 19 && j > 2 && j < 19 && (i*13+j*7)%31 == 3 {
                let (a,b) = project(x+0.5,y+0.5,z);
                faces.push(Face { vertices: vec![(a-0.6,b),(a+0.6,b),(a+0.6,b-18.),(a-0.6,b-18.)],
                    color: 0x5a7c8bb3 });
                faces.push(Face { vertices: vec![(a-7.,b-9.),(a-7.,b-15.),(a-3.,b-15.),(a-3.,b-23.),
                    (a+3.,b-23.),(a+3.,b-15.),(a+7.,b-15.),(a+7.,b-9.)], color: 0x7598a6c7 });
            }
        }
    }
    faces
}

fn geometry() -> &'static [Face] {
    static FACES: OnceLock<Vec<Face>> = OnceLock::new();
    FACES.get_or_init(generate)
}

pub(crate) fn illustration(width: f32) -> impl IntoElement {
    let faces = geometry();
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let scale = width / 396.;
            for face in faces {
                let mut path = PathBuilder::fill();
                for (i, &(x,y)) in face.vertices.iter().enumerate() {
                    let point = bounds.origin + point(px(x * scale), px(y * scale));
                    if i == 0 { path.move_to(point); } else { path.line_to(point); }
                }
                path.close();
                if let Ok(path) = path.build() { window.paint_path(path, rgba(face.color)); }
            }
        },
    ).w(px(width)).h(px(width * 250. / 396.)).flex_shrink_0()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn illustration_is_deterministic_and_bounded() {
        let faces = generate();
        assert_eq!(faces, generate());
        assert!(!faces.is_empty() && faces.len() < N*N*4);
        for face in faces {
            assert!(face.vertices.len() >= 3);
            assert!(face.vertices.iter().all(|&(x,y)| x.is_finite() && y.is_finite()
                && (0.0..=396.0).contains(&x) && (0.0..=250.0).contains(&y)));
        }
    }

    #[test]
    fn geometry_is_reused_without_a_growing_cache() {
        assert!(std::ptr::eq(geometry(), geometry()));
    }
}