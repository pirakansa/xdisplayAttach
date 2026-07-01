use crate::{AttachError, Result, RotationRequest};
use x11rb::protocol::xinput::{DeviceClassData, XIDeviceInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Geometry {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Geometry {
    pub const fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub(super) fn coordinate_transformation_matrix(
    root: Geometry,
    output: Geometry,
    rotation: RotationRequest,
) -> Result<[f32; 9]> {
    if root.width == 0 || root.height == 0 {
        return Err(AttachError::randr(
            "root geometry must have positive width and height",
        ));
    }

    let root_width = f32::from(root.width);
    let root_height = f32::from(root.height);
    let output_width = f32::from(output.width) / root_width;
    let output_height = f32::from(output.height) / root_height;
    let output_x = f32::from(output.x - root.x) / root_width;
    let output_y = f32::from(output.y - root.y) / root_height;

    Ok(match rotation {
        RotationRequest::Left => [
            0.0,
            -output_width,
            output_x + output_width,
            output_height,
            0.0,
            output_y,
            0.0,
            0.0,
            1.0,
        ],
        RotationRequest::Inverted => [
            -output_width,
            0.0,
            output_x + output_width,
            0.0,
            -output_height,
            output_y + output_height,
            0.0,
            0.0,
            1.0,
        ],
        RotationRequest::Right => [
            0.0,
            output_width,
            output_x,
            -output_height,
            0.0,
            output_y + output_height,
            0.0,
            0.0,
            1.0,
        ],
        RotationRequest::Normal => [
            output_width,
            0.0,
            output_x,
            0.0,
            output_height,
            output_y,
            0.0,
            0.0,
            1.0,
        ],
    })
}

pub(super) fn touch_device(device: &XIDeviceInfo) -> bool {
    device.enabled
        && device
            .classes
            .iter()
            .any(|class| matches!(class.data, DeviceClassData::Touch(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::protocol::xinput::{
        DeviceClass, DeviceClassDataKey, DeviceClassDataTouch, DeviceId, DeviceType, TouchMode,
    };

    #[test]
    fn calculates_coordinate_transformation_matrix_for_output_relative_to_root() {
        let root = Geometry::new(0, 0, 3840, 1080);
        let output = Geometry::new(1920, 0, 1920, 1080);

        let matrix =
            coordinate_transformation_matrix(root, output, RotationRequest::Normal).unwrap();

        assert_eq!(matrix, [0.5, 0.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn calculates_coordinate_transformation_matrix_for_rotated_output() {
        let root = Geometry::new(0, 0, 3840, 2160);
        let output = Geometry::new(1920, 0, 1080, 1920);

        assert_eq!(
            coordinate_transformation_matrix(root, output, RotationRequest::Left).unwrap(),
            [0.0, -0.28125, 0.78125, 0.8888889, 0.0, 0.0, 0.0, 0.0, 1.0,]
        );
        assert_eq!(
            coordinate_transformation_matrix(root, output, RotationRequest::Inverted).unwrap(),
            [-0.28125, 0.0, 0.78125, 0.0, -0.8888889, 0.8888889, 0.0, 0.0, 1.0,]
        );
        assert_eq!(
            coordinate_transformation_matrix(root, output, RotationRequest::Right).unwrap(),
            [0.0, 0.28125, 0.5, -0.8888889, 0.0, 0.8888889, 0.0, 0.0, 1.0,]
        );
    }

    #[test]
    fn rejects_coordinate_transformation_matrix_for_invalid_root() {
        let output = Geometry::new(0, 0, 1920, 1080);

        assert!(coordinate_transformation_matrix(
            Geometry::new(0, 0, 0, 1080),
            output,
            RotationRequest::Normal
        )
        .is_err());
        assert!(coordinate_transformation_matrix(
            Geometry::new(0, 0, 1920, 0),
            output,
            RotationRequest::Normal
        )
        .is_err());
    }

    #[test]
    fn selects_only_enabled_touch_devices() {
        assert!(touch_device(&test_xi_device(
            true,
            vec![DeviceClassData::Touch(DeviceClassDataTouch {
                mode: TouchMode::DIRECT,
                num_touches: 10,
            })],
        )));
        assert!(!touch_device(&test_xi_device(
            false,
            vec![DeviceClassData::Touch(DeviceClassDataTouch {
                mode: TouchMode::DIRECT,
                num_touches: 10,
            })],
        )));
        assert!(!touch_device(&test_xi_device(
            true,
            vec![DeviceClassData::Key(DeviceClassDataKey {
                keys: Vec::new()
            })],
        )));
    }

    fn test_xi_device(enabled: bool, classes: Vec<DeviceClassData>) -> XIDeviceInfo {
        XIDeviceInfo {
            deviceid: DeviceId::from(1_u16),
            type_: DeviceType::SLAVE_POINTER,
            attachment: DeviceId::from(0_u16),
            enabled,
            name: b"test device".to_vec(),
            classes: classes
                .into_iter()
                .map(|data| DeviceClass {
                    len: 2,
                    sourceid: DeviceId::from(1_u16),
                    data,
                })
                .collect(),
        }
    }
}
