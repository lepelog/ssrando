use core::{
    ffi::{c_char, c_int, c_uint, c_ushort, c_void},
    mem::{align_of, size_of, MaybeUninit},
    ptr::{addr_of_mut, null_mut},
};

use cstr::cstr;

#[repr(C)]
pub struct OSAlarm {
    // treat data as opaque for now
    data: [u8; 44],
}

impl OSAlarm {
    pub const fn new() -> Self {
        OSAlarm { data: [0; 44] }
    }
}

extern "C" {
    pub fn ss_printf(string: *const c_char, ...);
    pub fn OSSetPeriodicAlarm(
        alarm: *mut OSAlarm,
        start: u64,
        period: u64,
        callback: extern "C" fn(*mut OSAlarm),
    );
    pub fn OSInsertAlarm(alarm: *mut OSAlarm, wait: u64, callback: extern "C" fn(*mut OSAlarm));
    fn OSGetTick() -> u32;
}

pub fn get_time_base() -> u32 {
    let reg = unsafe { (0x800000F8 as *const u32).read_volatile() };
    reg / 4000
}

pub fn os_get_tick() -> u32 {
    unsafe { OSGetTick() }
}

#[repr(C)]
pub struct OSThread {
    data: [u8; 792],
}

impl OSThread {
    pub const fn new() -> Self {
        OSThread { data: [0; 792] }
    }
}

#[repr(C)]
pub struct Ioctlv<const ARGC: usize> {
    args: [u32; ARGC],
}

extern "C" {
    pub fn OSThreadCreate(
        thread: *mut OSThread,
        entrypoint: extern "C" fn(*mut c_void) -> c_int,
        entrypoint_data: *mut c_void,
        stack_start: *mut u8,
        stack_size: c_uint,
        priority: c_int,
        flags: c_ushort,
    ) -> c_int;
    pub fn OSThreadStart(thread: *mut OSThread) -> c_int;
    pub fn OSYieldThread();

    pub fn IOS_Open(path: *const c_char, mode: c_int) -> c_int;
    pub fn IOS_OpenAsync(
        path: *const c_char,
        mode: c_int,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    pub fn IOS_Ioctlv(
        fd: c_int,
        cmd: c_int,
        in_cnt: c_int,
        out_cnt: c_int,
        ioctlv: *mut c_void,
    ) -> c_int;
    pub fn IOS_IoctlvAsync(
        fd: c_int,
        cmd: c_int,
        in_cnt: c_int,
        out_cnt: c_int,
        ioctlv: *mut c_void,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    pub fn IOS_IoctlAsync(
        fd: c_int,
        cmd: c_int,
        in_buf: *mut c_void,
        in_len: c_int,
        out_buf: *mut c_void,
        out_len: c_int,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    pub fn IOS_Close(fd: c_int) -> c_int;
    pub fn IOS_CloseAsync(
        fd: c_int,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    pub fn iosAllocAligned(heap: *const c_void, size: c_uint, align: c_int) -> *mut u8;
    pub fn iosFree(heap: *const c_void, ptr: *mut u8);
    pub static IOS_HEAP: *const c_void;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InitChainState {
    OpenManageFd,
    GetLinkStatus,
    CheckMac,
    CloseManageFd,
    OpenTopFd,
    OpenRequestFd,
    NWC24Startup, // needed?
    SocketStartup,
    CheckIp,
}

impl InitChainState {
    pub fn get_name(&self) -> *const c_char {
        use InitChainState::*;
        match self {
            OpenManageFd => cstr!("OpenManageFd"),
            GetLinkStatus => cstr!("GetLinkStatus"),
            CheckMac => cstr!("CheckMac"),
            CloseManageFd => cstr!("CloseManageFd"),
            OpenTopFd => cstr!("OpenTopFd"),
            OpenRequestFd => cstr!("OpenRequestFd"),
            NWC24Startup => cstr!("NWC24Startup"),
            SocketStartup => cstr!("SocketStartup"),
            CheckIp => cstr!("CheckIp"),
        }
        .as_ptr()
    }
    pub fn get_name_str(&self) -> &'static str {
        use InitChainState::*;
        match self {
            OpenManageFd => "OpenManageFd",
            GetLinkStatus => "GetLinkStatus",
            CheckMac => "CheckMac",
            CloseManageFd => "CloseManageFd",
            OpenTopFd => "OpenTopFd",
            OpenRequestFd => "OpenRequestFd",
            NWC24Startup => "NWC24Startup",
            SocketStartup => "SocketStartup",
            CheckIp => "CheckIp",
        }
    }
}

#[derive(Clone, Copy)]
struct IoctlvMac {
    buf_ptr: *mut u8,
    buf_len: u32,
    mac_ptr: *mut u8,
    mac_len: u32,
}

#[derive(Clone, Copy)]
struct IoctlvLinkStatus {
    buf_ptr: *mut u8,
    buf_len: u32,
}

#[repr(align(0x20))]
struct AlignedBuf<const SIZE: usize> {
    buf: [u8; SIZE],
}

impl<const SIZE: usize> Default for AlignedBuf<SIZE> {
    fn default() -> Self {
        Self { buf: [0; SIZE] }
    }
}

#[repr(C, align(0x20))]
union IoctlvUnion {
    none:      (),
    mac:       IoctlvMac,
    netstatus: IoctlvLinkStatus,
}

#[repr(C, align(0x20))]
pub struct NetInitData {
    ioctlv:          IoctlvUnion,
    pub manage_fd:   c_int,
    pub top_fd:      c_int,
    pub request_fd:  c_int,
    pub ip:          u32,
    // if this is None, this is a retry
    pub last_result: Option<InitChainState>,
    pub state:       InitChainState,
    pub mac_buf:     [u8; 6],
    pub retry_alarm: OSAlarm,
    pub last_err:    c_int,
    pub retry_count: u32,
    cmd_buf:         AlignedBuf<0x20>,
}

pub extern "C" fn net_init_retry_alarm_callback(alarm: *mut OSAlarm) {
    let init_data =
        unsafe { field_offset::offset_of!(NetInitData => retry_alarm).unapply_ptr_mut(alarm) };
    net_init_callback(0, init_data.cast())
}

fn retry_timeout() -> u64 {
    (get_time_base() * 4000).into()
}

#[link_section = ".data"]
pub static mut INIT_CHAIN_DATA_PTR: usize = 0;

#[no_mangle]
pub fn do_init_chain() {
    let init_data = unsafe {
        &mut *(iosAllocAligned(
            IOS_HEAP,
            size_of::<NetInitData>() as u32,
            align_of::<NetInitData>() as i32,
        ) as *mut MaybeUninit<NetInitData>)
    };
    init_data.write(NetInitData {
        ioctlv:      IoctlvUnion { none: () },
        manage_fd:   -1,
        top_fd:      -1,
        request_fd:  -1,
        ip:          0,
        last_result: None,
        state:       InitChainState::OpenManageFd,
        mac_buf:     [0; 6],
        last_err:    0,
        retry_count: 0,
        retry_alarm: OSAlarm::new(),
        cmd_buf:     AlignedBuf::default(),
    });
    unsafe { INIT_CHAIN_DATA_PTR = init_data.as_ptr() as usize };
    net_init_callback(0, init_data.as_mut_ptr().cast());
}

pub extern "C" fn net_init_callback(result: c_int, usr_data: *mut c_void) {
    let init_data = unsafe { &mut *(usr_data.cast::<NetInitData>()) };
    unsafe {
        ss_printf(
            cstr!("getting called with %d in state %s\n").as_ptr(),
            result,
            init_data.state.get_name(),
        )
    };
    use InitChainState::*;
    // the ip comes in the result...
    // ip = 0 is error
    let was_error = match init_data.last_result {
        Some(CheckIp) => result == 0,
        Some(NWC24Startup) => result != -15, // happens when it's already started
        _ => result < 0 && result > -0x8000,
    };
    if was_error {
        // error state, retry in a bit
        if let Some(last_action) = init_data.last_result.as_ref() {
            unsafe {
                ss_printf(
                    cstr!("%s failed: %d\n").as_ptr(),
                    last_action.get_name(),
                    result,
                )
            };
        }
        init_data.retry_count += 1;
        init_data.last_err = result;
        init_data.last_result = None;
        // retry after timeout
        unsafe {
            OSInsertAlarm(
                &mut init_data.retry_alarm as *mut OSAlarm,
                retry_timeout(),
                net_init_retry_alarm_callback,
            )
        };
        return;
    } else {
        // success!
        init_data.retry_count = 0;
        match init_data.last_result {
            Some(OpenManageFd) => {
                init_data.manage_fd = result;
                init_data.state = GetLinkStatus;
            },
            Some(GetLinkStatus) => {
                // I think it should do something here?
                init_data.state = CheckMac;
            },
            Some(CheckMac) => {
                init_data.state = CloseManageFd;
            },
            Some(CloseManageFd) => {
                init_data.state = OpenTopFd;
            },
            Some(OpenTopFd) => {
                init_data.top_fd = result;
                init_data.state = OpenRequestFd;
            },
            Some(OpenRequestFd) => {
                init_data.request_fd = result;
                init_data.state = NWC24Startup;
            },
            Some(NWC24Startup) => {
                init_data.state = SocketStartup;
            },
            Some(SocketStartup) => {
                init_data.state = CheckIp;
            },
            Some(CheckIp) => {
                init_data.ip = result as u32;
                // we're DONE
                // leak the memory for now, will be accessed by others
                // unsafe { iosFree(IOS_HEAP, (init_data as *mut NetInitData).cast()) };
                return;
            },
            // None, this mean we're retrying or at the start
            None => (),
        }
    }
    init_data.last_result = Some(init_data.state);
    match init_data.state {
        OpenManageFd => {
            let result = unsafe {
                IOS_OpenAsync(
                    cstr!("/dev/net/ncd/manage").as_ptr(),
                    0,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(
                        cstr!("could not call get link status: %d\n").as_ptr(),
                        result,
                    );
                }
            }
        },
        GetLinkStatus => {
            init_data.ioctlv.netstatus.buf_len = 0x20;
            init_data.ioctlv.netstatus.buf_ptr = addr_of_mut!((*init_data).cmd_buf).cast();
            let result = unsafe {
                IOS_IoctlvAsync(
                    init_data.manage_fd,
                    7, // IOCTL_NCD_GETLINKSTATUS
                    0,
                    1,
                    usr_data,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(
                        cstr!("could not call get link status: %d\n").as_ptr(),
                        result,
                    );
                }
            }
        },
        CheckMac => {
            init_data.ioctlv.mac.mac_len = 6;
            init_data.ioctlv.mac.mac_ptr = addr_of_mut!((*init_data).mac_buf).cast();
            init_data.ioctlv.mac.buf_len = 0x20;
            init_data.ioctlv.mac.buf_ptr = addr_of_mut!((*init_data).cmd_buf).cast();
            let result = unsafe {
                IOS_IoctlvAsync(
                    init_data.manage_fd,
                    8, // MAC
                    0,
                    2,
                    usr_data,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(cstr!("sending mac: %d\n").as_ptr(), result);
                }
            }
        },
        CloseManageFd => {
            let result =
                unsafe { IOS_CloseAsync(init_data.manage_fd, net_init_callback, usr_data) };
            if result < 0 {
                unsafe {
                    ss_printf(cstr!("sending close: %d\n").as_ptr(), result);
                }
            }
        },
        OpenRequestFd => {
            let result = unsafe {
                IOS_OpenAsync(
                    cstr!("/dev/net/kd/request").as_ptr(),
                    0,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(cstr!("sending open request: %d\n").as_ptr(), result);
                }
            }
        },
        OpenTopFd => {
            let result = unsafe {
                IOS_OpenAsync(
                    cstr!("/dev/net/ip/top").as_ptr(),
                    0,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(cstr!("sending open top: %d\n").as_ptr(), result);
                }
            }
        },
        NWC24Startup => {
            let result = unsafe {
                IOS_IoctlAsync(
                    init_data.request_fd,
                    6, // IOCTL_NWC24_STARTUP
                    null_mut(),
                    0,
                    addr_of_mut!((*init_data).cmd_buf).cast(),
                    0x20,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(cstr!("sending nwc24 startup: %d\n").as_ptr(), result);
                }
            }
        },
        SocketStartup => {
            let result = unsafe {
                IOS_IoctlAsync(
                    init_data.top_fd,
                    31, // IOCTL_SO_STARTUP
                    null_mut(),
                    0,
                    null_mut(),
                    0,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(cstr!("socket startup: %d\n").as_ptr(), result);
                }
            }
        },
        CheckIp => {
            let result = unsafe {
                IOS_IoctlAsync(
                    init_data.top_fd,
                    16, // IOCTL_SO_GETHOSTID
                    null_mut(),
                    0,
                    null_mut(),
                    0,
                    net_init_callback,
                    usr_data,
                )
            };
            if result < 0 {
                unsafe {
                    ss_printf(cstr!("check ip: %d\n").as_ptr(), result);
                }
            }
        },
    }
}
