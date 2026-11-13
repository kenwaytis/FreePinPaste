//! Native window management. All desktop positions and sizes are physical pixels.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
pub use linux::Platform;
#[cfg(target_os = "windows")]
pub use windows::Platform;
#[cfg(target_os = "windows")]
pub(crate) use windows::capture;

#[cfg(target_os = "linux")]
pub type WindowId = u32;
#[cfg(target_os = "windows")]
pub type WindowId = isize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Desktop {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Desktop {
    pub fn clamp_position(self, x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
        (
            x.clamp(
                self.x as f32,
                self.x as f32 + (self.width as f32 - width).max(0.),
            ),
            y.clamp(
                self.y as f32,
                self.y as f32 + (self.height as f32 - height).max(0.),
            ),
        )
    }

    /// Translate an editor's logical point to virtual-desktop physical pixels.
    pub fn screen_point(self, point: (f32, f32), viewport: (f32, f32)) -> (f32, f32) {
        (
            self.x as f32 + point.0 * self.width as f32 / viewport.0,
            self.y as f32 + point.1 * self.height as f32 / viewport.1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_origin_and_scaled_editor_coordinates() {
        let desktop = Desktop {
            x: -1920,
            y: -1080,
            width: 4480,
            height: 2520,
        };
        assert_eq!(
            desktop.screen_point((0., 0.), (2986.6667, 1680.)),
            (-1920., -1080.)
        );
        assert_eq!(
            desktop.screen_point((1493.3334, 840.), (2986.6667, 1680.)),
            (320., 180.)
        );
        assert_eq!(
            desktop.clamp_position(-2000., -1200., 400., 300.),
            (-1920., -1080.)
        );
        assert_eq!(
            desktop.clamp_position(3000., 2000., 400., 300.),
            (2160., 1140.)
        );
    }
}
