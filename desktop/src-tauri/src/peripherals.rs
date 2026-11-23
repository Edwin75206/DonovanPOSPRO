use crate::error::Result;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeripheralStatus {
    barcode_readers: Option<u32>,
    usb_printers: Option<u32>,
}

#[cfg(windows)]
pub fn status() -> Result<PeripheralStatus> {
    use crate::error::AppError;
    use std::{mem::size_of, ptr};
    use windows_sys::{
        Win32::{
            Devices::DeviceAndDriverInstallation::{
                DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA,
                SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
            },
            Foundation::{ERROR_NO_MORE_ITEMS, GetLastError, INVALID_HANDLE_VALUE},
        },
        core::GUID,
    };

    struct DeviceInfoSet(HDEVINFO);
    impl Drop for DeviceInfoSet {
        fn drop(&mut self) {
            unsafe { SetupDiDestroyDeviceInfoList(self.0) };
        }
    }
    fn query_error(code: u32) -> AppError {
        AppError::internal(
            "PERIPHERALS",
            "No pudimos consultar los dispositivos conectados a Windows.",
            code,
        )
    }

    fn count_present(class: &GUID, flags: u32) -> Result<u32> {
        let handle = unsafe {
            SetupDiGetClassDevsW(class, ptr::null(), ptr::null_mut(), DIGCF_PRESENT | flags)
        };
        if handle as usize == INVALID_HANDLE_VALUE as usize {
            return Err(query_error(unsafe { GetLastError() }));
        }
        let devices = DeviceInfoSet(handle);
        let mut count = 0;
        let mut device = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        while unsafe { SetupDiEnumDeviceInfo(devices.0, count, &mut device) } != 0 {
            count += 1;
        }
        match unsafe { GetLastError() } {
            ERROR_NO_MORE_ITEMS => Ok(count),
            error => Err(query_error(error)),
        }
    }

    // Windows assigns this setup class to dedicated POS scanners. Readers in
    // keyboard mode are tested through the same input path as checkout.
    const BARCODE_SCANNER_CLASS: GUID = GUID::from_u128(0xc243ffbd_3afc_45e9_b3d3_2ba18bc7ebc5);
    // GUID_DEVINTERFACE_USBPRINT from the Windows SDK's usbprint.h.
    const USB_PRINTER_INTERFACE: GUID = GUID::from_u128(0x28d78fad_5a12_11d1_ae5b_0000f803a8c2);
    Ok(PeripheralStatus {
        barcode_readers: Some(count_present(&BARCODE_SCANNER_CLASS, 0)?),
        usb_printers: Some(count_present(&USB_PRINTER_INTERFACE, DIGCF_DEVICEINTERFACE)?),
    })
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn windows_returns_peripheral_counts_even_without_connected_hardware() {
        let devices = super::status().expect("query Windows devices");
        assert!(devices.barcode_readers.is_some());
        assert!(devices.usb_printers.is_some());
    }
}

#[cfg(not(windows))]
pub fn status() -> Result<PeripheralStatus> {
    Ok(PeripheralStatus {
        barcode_readers: None,
        usb_printers: None,
    })
}
