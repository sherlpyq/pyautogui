#![no_std]

use core::ffi::c_void;
use core::panic::PanicInfo;

pub type PVOID = *mut core::ffi::c_void;
pub type NTSTATUS = i32;

#[repr(C)]
pub struct DRIVER_OBJECT {
    pub Type: i16,
    pub Size: i16,
    pub _align1: i32,
    pub DeviceObject: PVOID,
    pub Flags: u32,
    pub _align2: u32,
    pub DriverStart: PVOID,
    pub DriverSize: u32,
    pub _align3: u32,
    pub DriverSection: PVOID,
    pub DriverExtension: PVOID,
    pub DriverName: UNICODE_STRING,
    pub HardwareDatabase: PVOID,
    pub FastIoDispatch: PVOID,
    pub DriverInit: PVOID,
    pub DriverStartIo: PVOID,
    pub DriverUnload: Option<unsafe extern "system" fn(DriverObject: *mut DRIVER_OBJECT)>,
    pub MajorFunction: [PVOID; 28],
}

#[repr(C)]
pub struct DEVICE_OBJECT {
    pub Type: i16,
    pub Size: u16,
    pub ReferenceCount: i32,
    pub DriverObject: *mut DRIVER_OBJECT,
    pub NextDevice: *mut DEVICE_OBJECT,
    pub AssociatedDevice: *mut DEVICE_OBJECT,
    pub CurrentIrp: *mut c_void,
    pub Timer: *mut c_void,
    pub Flags: u32,
    pub Characteristics: u32,
    pub Vpb: *mut c_void,
    pub DeviceExtension: PVOID,
    pub DeviceType: u32,
    pub StackSize: i8,
}

#[repr(C)]
pub struct UNICODE_STRING {
    pub Length: u16,
    pub MaximumLength: u16,
    pub Buffer: *mut u16,
}

#[repr(C)]
pub struct IO_STATUS_BLOCK {
    pub Status: NTSTATUS,
    pub Information: usize,
}

#[repr(C)]
pub struct IRP {
    pub Type: i16,
    pub Size: u16,
    pub MdlAddress: PVOID,
    pub Flags: u32,
    pub AssociatedIrp: AssociatedIrpUnion,
    pub ThreadListEntry: [PVOID; 2],
    pub IoStatus: IO_STATUS_BLOCK,
    pub RequestorMode: i8,
    pub PendingReturned: u8,
    pub StackCount: i8,
    pub CurrentLocation: i8,
    pub Cancel: u8,
    pub CancelIrql: u8,
    pub ApcBypassCount: i8,
    pub UserAddress: PVOID,
    pub UserEvent: PVOID,
    pub Overlay: [PVOID; 2],
    pub CancelRoutine: PVOID,
    pub UserBuffer: PVOID,
    pub Tail: IrpTailUnion,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union AssociatedIrpUnion {
    pub MasterIrp: *mut IRP,
    pub SystemBuffer: PVOID,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union IrpTailUnion {
    pub Overlay: IrpTailOverlay,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IrpTailOverlay {
    pub CurrentStackLocation: *mut IO_STACK_LOCATION,
    pub PacketType: u32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IO_STACK_LOCATION {
    pub MajorFunction: u8,
    pub MinorFunction: u8,
    pub Flags: u8,
    pub Control: u8,
    pub Parameters: IoStackLocationParameters,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union IoStackLocationParameters {
    pub DeviceIoControl: DeviceIoControlParameters,
    pub Others: [PVOID; 4],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DeviceIoControlParameters {
    pub OutputBufferLength: u32,
    pub InputBufferLength: u32,
    pub IoControlCode: u32,
    pub Type3InputBuffer: PVOID,
}

#[repr(C)]
pub struct KEYBOARD_INPUT_DATA {
    pub UnitId: u16,
    pub MakeCode: u16,
    pub Flags: u16,
    pub Reserved: u16,
    pub ExtraInformation: u32,
}

#[repr(C)]
pub struct MOUSE_INPUT_DATA {
    pub UnitId: u16,
    pub Flags: u16,
    pub ButtonFlags: u16,
    pub ButtonData: u16,
    pub RawButtons: u32,
    pub LastX: i32,
    pub LastY: i32,
    pub ExtraInformation: u32,
}

type KeyboardClassServiceCallback = unsafe extern "system" fn(
    DeviceObject: *mut c_void,
    InputDataStart: *mut KEYBOARD_INPUT_DATA,
    InputDataEnd: *mut KEYBOARD_INPUT_DATA,
    InputDataConsumed: *mut u32,
);

type MouseClassServiceCallback = unsafe extern "system" fn(
    DeviceObject: *mut c_void,
    InputDataStart: *mut MOUSE_INPUT_DATA,
    InputDataEnd: *mut MOUSE_INPUT_DATA,
    InputDataConsumed: *mut u32,
);

static mut G_KBD_CALLBACK: Option<KeyboardClassServiceCallback> = None;
static mut G_KBD_DEVICE: *mut c_void = core::ptr::null_mut();
static mut G_MOU_CALLBACK: Option<MouseClassServiceCallback> = None;
static mut G_MOU_DEVICE: *mut c_void = core::ptr::null_mut();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[link(name = "ntoskrnl")]
extern "system" {
    fn RtlInitUnicodeString(DestinationString: *mut UNICODE_STRING, SourceString: *const u16);
    fn IoCreateDevice(
        DriverObject: *mut DRIVER_OBJECT,
        DeviceExtensionSize: u32,
        DeviceName: *mut UNICODE_STRING,
        DeviceType: u32,
        DeviceCharacteristics: u32,
        Exclusive: u8,
        DeviceObject: *mut *mut DEVICE_OBJECT,
    ) -> NTSTATUS;
    fn IoCreateSymbolicLink(
        SymbolicLinkName: *mut UNICODE_STRING,
        DeviceName: *mut UNICODE_STRING,
    ) -> NTSTATUS;
    fn IoDeleteSymbolicLink(SymbolicLinkName: *mut UNICODE_STRING) -> NTSTATUS;
    fn IoDeleteDevice(DeviceObject: *mut DEVICE_OBJECT);
    fn IofCompleteRequest(Irp: *mut IRP, PriorityBoost: i8);
    fn DbgPrint(Format: *const u8, ...) -> i32;
    fn IoGetDeviceObjectPointer(
        ObjectName: *mut UNICODE_STRING,
        DesiredAccess: u32,
        FileObject: *mut *mut c_void,
        DeviceObject: *mut *mut DEVICE_OBJECT,
    ) -> NTSTATUS;
    fn ObfDereferenceObject(Object: *mut c_void);
}

static mut G_DEVICE_OBJECT: *mut DEVICE_OBJECT = core::ptr::null_mut();

pub unsafe extern "system" fn driver_unload(driver_object: *mut DRIVER_OBJECT) {
    let mut sym_link = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: core::ptr::null_mut(),
    };
    let sym_link_name: [u16; 21] = [
        92, 68, 111, 115, 68, 101, 118, 105, 99, 101, 115, 92, 77, 121, 68, 114, 105, 118, 101, 114, 0,
    ];
    RtlInitUnicodeString(&mut sym_link, sym_link_name.as_ptr());
    IoDeleteSymbolicLink(&mut sym_link);
    if !G_DEVICE_OBJECT.is_null() {
        IoDeleteDevice(G_DEVICE_OBJECT);
    }
}

pub unsafe extern "system" fn dispatch_create_close(
    _device_object: *mut DEVICE_OBJECT,
    irp: *mut IRP,
) -> NTSTATUS {
    (*irp).IoStatus.Status = 0;
    (*irp).IoStatus.Information = 0;
    IofCompleteRequest(irp, 0);
    0
}

#[repr(C)]
pub struct InputEvent {
    pub event_type: u32,
    pub mouse_flags: u16,
    pub button_flags: u16,
    pub x: i32,
    pub y: i32,
    pub keyboard_flags: u32,
}

pub unsafe extern "system" fn dispatch_device_control(
    _device_object: *mut DEVICE_OBJECT,
    irp: *mut IRP,
) -> NTSTATUS {
    let stack = (*irp).Tail.Overlay.CurrentStackLocation;
    let ioctl = (*stack).Parameters.DeviceIoControl.IoControlCode;
    let input_len = (*stack).Parameters.DeviceIoControl.InputBufferLength;
    let system_buffer = (*irp).AssociatedIrp.SystemBuffer;

    let mut status = 0i32;
    let mut info = 0usize;

    if ioctl == 0x00222000 {
        let event_size = core::mem::size_of::<InputEvent>() as u32;
        if input_len >= event_size && !system_buffer.is_null() {
            let event = &*(system_buffer as *const InputEvent);
            if event.event_type == 1 {
                if let (Some(cb), kbd) = (G_KBD_CALLBACK, G_KBD_DEVICE) {
                    if !kbd.is_null() {
                        let mut data = KEYBOARD_INPUT_DATA {
                            UnitId: 0,
                            MakeCode: event.y as u16,
                            Flags: event.keyboard_flags as u16,
                            Reserved: 0,
                            ExtraInformation: 0,
                        };
                        let mut consumed = 0u32;
                        cb(
                            kbd,
                            &mut data,
                            (&mut data as *mut KEYBOARD_INPUT_DATA).add(1),
                            &mut consumed,
                        );
                    }
                }
                status = 0;
                info = event_size as usize;
            } else if event.event_type == 0 {
                if let (Some(cb), mou) = (G_MOU_CALLBACK, G_MOU_DEVICE) {
                    if !mou.is_null() {
                        let mut data = MOUSE_INPUT_DATA {
                            UnitId: 0,
                            Flags: event.mouse_flags,
                            ButtonFlags: event.button_flags,
                            ButtonData: 0,
                            RawButtons: 0,
                            LastX: event.x,
                            LastY: event.y,
                            ExtraInformation: 0,
                        };
                        let mut consumed = 0u32;
                        cb(
                            mou,
                            &mut data,
                            (&mut data as *mut MOUSE_INPUT_DATA).add(1),
                            &mut consumed,
                        );
                    }
                }
                status = 0;
                info = event_size as usize;
            } else {
                status = -1073741811;
            }
        } else {
            status = -1073741811;
        }
    } else {
        status = -1073741808;
    }

    (*irp).IoStatus.Status = status;
    (*irp).IoStatus.Information = info;
    IofCompleteRequest(irp, 0);
    status
}

#[no_mangle]
pub unsafe extern "system" fn DriverEntry(
    driver_object: *mut DRIVER_OBJECT,
    _registry_path: PVOID,
) -> NTSTATUS {
    (*driver_object).DriverUnload = Some(driver_unload);

    let mut dev_name = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: core::ptr::null_mut(),
    };
    let dev_name_str: [u16; 17] = [
        92, 68, 101, 118, 105, 99, 101, 92, 77, 121, 68, 114, 105, 118, 101, 114, 0,
    ];
    RtlInitUnicodeString(&mut dev_name, dev_name_str.as_ptr());

    let mut device_object = core::ptr::null_mut();
    let status = IoCreateDevice(
        driver_object,
        0,
        &mut dev_name,
        0x00000022,
        0,
        0,
        &mut device_object,
    );

    if status < 0 {
        return status;
    }

    G_DEVICE_OBJECT = device_object;

    let mut sym_link = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: core::ptr::null_mut(),
    };
    let sym_link_str: [u16; 21] = [
        92, 68, 111, 115, 68, 101, 118, 105, 99, 101, 115, 92, 77, 121, 68, 114, 105, 118, 101, 114, 0,
    ];
    RtlInitUnicodeString(&mut sym_link, sym_link_str.as_ptr());

    let status = IoCreateSymbolicLink(&mut sym_link, &mut dev_name);
    if status < 0 {
        IoDeleteDevice(device_object);
        G_DEVICE_OBJECT = core::ptr::null_mut();
        return status;
    }

    (*driver_object).MajorFunction[0] = dispatch_create_close as PVOID;
    (*driver_object).MajorFunction[2] = dispatch_create_close as PVOID;
    (*driver_object).MajorFunction[14] = dispatch_device_control as PVOID;

    let mut kbd_name = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: core::ptr::null_mut(),
    };
    let kbd_name_str: [u16; 23] = [
        92, 68, 101, 118, 105, 99, 101, 92, 75, 101, 121, 98, 111, 97, 114, 100, 67, 108, 97, 115, 115, 48, 0,
    ];
    RtlInitUnicodeString(&mut kbd_name, kbd_name_str.as_ptr());
    let mut kbd_file_obj: *mut c_void = core::ptr::null_mut();
    let mut kbd_device_obj: *mut DEVICE_OBJECT = core::ptr::null_mut();
    let kbd_status = IoGetDeviceObjectPointer(&mut kbd_name, 0x001F0000, &mut kbd_file_obj, &mut kbd_device_obj);
    if kbd_status >= 0 {
        let kbd_driver = (*kbd_device_obj).DriverObject;
        let kbd_start = (*kbd_driver).DriverStart as usize;
        let kbd_end = kbd_start + (*kbd_driver).DriverSize as usize;
        let dev_ext = (*kbd_device_obj).DeviceExtension as *const usize;
        for i in 0..32 {
            let p1 = *dev_ext.add(i);
            let p2 = *dev_ext.add(i + 1);
            if p1 > 0xFFFF800000000000 && p2 >= kbd_start && p2 < kbd_end {
                let type_val = *(p1 as *const i16);
                if type_val == 3 {
                    G_KBD_DEVICE = p1 as *mut c_void;
                    G_KBD_CALLBACK = Some(core::mem::transmute(p2));
                    break;
                }
            }
        }
        ObfDereferenceObject(kbd_file_obj);
    }

    let mut mou_name = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: core::ptr::null_mut(),
    };
    let mou_name_str: [u16; 22] = [
        92, 68, 101, 118, 105, 99, 101, 92, 80, 111, 105, 110, 116, 101, 114, 67, 108, 97, 115, 115, 48, 0,
    ];
    RtlInitUnicodeString(&mut mou_name, mou_name_str.as_ptr());
    let mut mou_file_obj: *mut c_void = core::ptr::null_mut();
    let mut mou_device_obj: *mut DEVICE_OBJECT = core::ptr::null_mut();
    let mou_status = IoGetDeviceObjectPointer(&mut mou_name, 0x001F0000, &mut mou_file_obj, &mut mou_device_obj);
    if mou_status >= 0 {
        let mou_driver = (*mou_device_obj).DriverObject;
        let mou_start = (*mou_driver).DriverStart as usize;
        let mou_end = mou_start + (*mou_driver).DriverSize as usize;
        let dev_ext = (*mou_device_obj).DeviceExtension as *const usize;
        for i in 0..32 {
            let p1 = *dev_ext.add(i);
            let p2 = *dev_ext.add(i + 1);
            if p1 > 0xFFFF800000000000 && p2 >= mou_start && p2 < mou_end {
                let type_val = *(p1 as *const i16);
                if type_val == 3 {
                    G_MOU_DEVICE = p1 as *mut c_void;
                    G_MOU_CALLBACK = Some(core::mem::transmute(p2));
                    break;
                }
            }
        }
        ObfDereferenceObject(mou_file_obj);
    }

    0
}
