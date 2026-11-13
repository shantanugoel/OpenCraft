//! The layout of a device model, as a message for a shell.
//!
//! A shell does not use the device layer (architecture rule 2). The engine
//! gives the shell the data that it needs to show the device: the surfaces
//! with their positions, and the controls with their places. The layout
//! contains data only, and no reference to the descriptor.

use opencraft_core::{ControlId, Descriptor, SettingsSupport, SurfaceId};

/// One surface of the layout. The position and the size are in pixels of the
/// display. See [`DeviceLayout::display_width`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceLayout {
    pub id: SurfaceId,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// One control of the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlLayout {
    pub id: ControlId,
    /// The place of the control among the controls of the same kind. Row 1
    /// is the top row. Column 1 is the left column.
    pub row: u8,
    pub column: u8,
    /// The surface of the control, if the control has a display. The control
    /// is at the position of this surface.
    pub surface: Option<SurfaceId>,
    /// The surface that shows feedback for this control.
    pub feedback_surface: Option<SurfaceId>,
}

/// The layout of one device model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceLayout {
    /// The name of the model.
    pub name: String,
    /// The size of the display on which the surfaces have their positions.
    pub display_width: u16,
    pub display_height: u16,
    pub surfaces: Vec<SurfaceLayout>,
    pub controls: Vec<ControlLayout>,
    /// The device settings that the model has. The shell shows only these
    /// settings.
    pub settings: SettingsSupport,
}

impl DeviceLayout {
    /// The layout of the first descriptor: the layout that the engine sends
    /// before the first connection. A shell test uses it, for example to
    /// check the starter profiles of the first start against the model.
    pub fn of_default_model() -> Self {
        Self::from(&opencraft_core::device::DESCRIPTORS[0])
    }

    pub fn surface(&self, id: SurfaceId) -> Option<&SurfaceLayout> {
        self.surfaces.iter().find(|s| s.id == id)
    }
}

impl From<&Descriptor> for DeviceLayout {
    fn from(descriptor: &Descriptor) -> Self {
        let (display_width, display_height) = descriptor.display_size();
        Self {
            name: descriptor.name.to_string(),
            display_width,
            display_height,
            surfaces: descriptor
                .surfaces
                .iter()
                .map(|s| SurfaceLayout {
                    id: s.id,
                    x: s.x,
                    y: s.y,
                    width: s.width,
                    height: s.height,
                })
                .collect(),
            controls: descriptor
                .controls
                .iter()
                .map(|c| ControlLayout {
                    id: c.id(),
                    row: c.position.row,
                    column: c.position.column,
                    surface: c.surface,
                    feedback_surface: c.feedback_surface(),
                })
                .collect(),
            settings: descriptor.settings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencraft_core::device::DESCRIPTORS;
    use opencraft_core::{ControlKind, SurfaceKind};

    #[test]
    fn the_layout_has_each_surface_and_each_control_of_the_descriptor() {
        for d in DESCRIPTORS {
            let layout = DeviceLayout::from(d);
            assert_eq!(layout.name, d.name);
            assert_eq!(
                (layout.display_width, layout.display_height),
                d.display_size()
            );
            assert_eq!(layout.surfaces.len(), d.surfaces.len());
            assert_eq!(layout.controls.len(), d.controls.len());
            assert_eq!(layout.settings, d.settings);
            for s in d.surfaces {
                let got = layout.surface(s.id).unwrap();
                assert_eq!(
                    (got.x, got.y, got.width, got.height),
                    (s.x, s.y, s.width, s.height)
                );
            }
            for (got, c) in layout.controls.iter().zip(d.controls) {
                assert_eq!(got.id, c.id());
                assert_eq!((got.row, got.column), (c.position.row, c.position.column));
                assert_eq!(got.surface, c.surface);
                assert_eq!(got.feedback_surface, c.feedback_surface());
            }
        }
    }

    #[test]
    fn a_key_of_the_layout_is_at_the_position_of_its_key_face() {
        let layout = DeviceLayout::from(&DESCRIPTORS[0]);
        let key = layout
            .controls
            .iter()
            .find(|c| c.id == ControlId::new(ControlKind::Key, 7))
            .unwrap();
        let face = layout.surface(key.surface.unwrap()).unwrap();
        assert_eq!(face.id, SurfaceId::new(SurfaceKind::KeyFace, 7));
        assert_eq!((key.row, key.column), (2, 2));
        assert_eq!((face.x, face.y), (176, 178));
    }
}
