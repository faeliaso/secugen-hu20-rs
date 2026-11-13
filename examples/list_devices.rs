use secugen_hu20_rs::list_devices;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("SECUGEN_USB_INVENTORY").is_some() {
        let devices = rusb::devices()?;
        eprintln!("libusb enumerated {} USB device(s)", devices.len());
        for device in devices.iter() {
            if let Ok(descriptor) = device.device_descriptor() {
                eprintln!(
                    "raw usb={}:{} vid={:04x} pid={:04x}",
                    device.bus_number(),
                    device.address(),
                    descriptor.vendor_id(),
                    descriptor.product_id(),
                );
                if descriptor.vendor_id() == 0x1162 && descriptor.product_id() == 0x2200 {
                    if let Ok(config) = device.active_config_descriptor() {
                        for interface in config.interfaces() {
                            for alternate in interface.descriptors() {
                                for endpoint in alternate.endpoint_descriptors() {
                                    eprintln!(
                                        "hu20 interface={} alt={} endpoint=0x{:02x} direction={:?} transfer={:?} max_packet={}",
                                        alternate.interface_number(),
                                        alternate.setting_number(),
                                        endpoint.address(),
                                        endpoint.direction(),
                                        endpoint.transfer_type(),
                                        endpoint.max_packet_size(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let devices = list_devices()?;
    if devices.is_empty() {
        println!("No SecuGen HU20 scanners (1162:2200) found.");
    }
    for device in devices {
        println!(
            "vid={:04x} pid={:04x} usb={} serial={} product={}",
            device.vendor_id,
            device.product_id,
            device.id,
            device.serial_number.as_deref().unwrap_or("unavailable"),
            device.product_name.as_deref().unwrap_or("unavailable"),
        );
    }
    Ok(())
}
