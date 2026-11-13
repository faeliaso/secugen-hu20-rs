use secugen_hu20_rs::{DeviceId, ParseDeviceIdError, RawFrame, ScannerError, FRAME_BYTES};

#[test]
fn device_id_parses_and_formats() {
    let id: DeviceId = "001:004".parse().unwrap();
    assert_eq!(id, DeviceId::new(1, 4));
    assert_eq!(id.to_string(), "1:4");
}

#[test]
fn invalid_device_id_is_an_error() {
    let error = "not-an-id".parse::<DeviceId>().unwrap_err();
    let _: &dyn std::error::Error = &error;
    assert!(error.to_string().contains("invalid device ID"));
}

#[test]
fn raw_frame_requires_exact_geometry() {
    assert!(RawFrame::new(vec![0; FRAME_BYTES]).is_ok());
    assert!(matches!(
        RawFrame::new(vec![0; FRAME_BYTES - 1]),
        Err(ScannerError::IncompleteFrame { .. })
    ));
}

#[test]
#[ignore = "requires a physically connected HU20 after protocol validation"]
fn physical_hu20_is_discoverable() {
    let devices = secugen_hu20_rs::list_devices().unwrap();
    assert!(!devices.is_empty());
}

#[test]
fn parse_error_type_is_public() {
    let _: ParseDeviceIdError = "1".parse::<DeviceId>().unwrap_err();
}
