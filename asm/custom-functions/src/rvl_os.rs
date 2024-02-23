use core::fmt::Write as _;
use core::net::Ipv4Addr;
use core::ptr::copy_nonoverlapping;
use core::{
    ffi::{c_char, c_int, c_uint, c_ushort, c_void},
    fmt::Debug,
    mem::{align_of, size_of, size_of_val},
    ptr::{addr_of, addr_of_mut, null, null_mut, NonNull},
    str::from_utf8,
};

use cstr::cstr;

use crate::console_print;
use crate::game::player;
use crate::rvl_mem::ios_allocate;
use crate::rvl_mem::{iosAllocAligned, iosFree, ios_free, IOS_HEAP};

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
    pub fn OSRegisterShutdownFunction(cb: extern "C" fn());
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
    pub fn IOS_Ioctl(
        fd: c_int,
        cmd: c_int,
        in_buf: *mut c_void,
        in_len: c_int,
        out_buf: *mut c_void,
        out_len: c_int,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    CreateSocket,
    ConnectSocket,
    SendSocketMessage,
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
            CreateSocket => cstr!("CreateSocket"),
            ConnectSocket => cstr!("ConnectSocket"),
            SendSocketMessage => cstr!("SendSocketMessage"),
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
            CreateSocket => "CreateSocket",
            ConnectSocket => "ConnectSocket",
            SendSocketMessage => "SendSocketMessage",
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct IoctlvMac {
    buf_ptr: *mut u8,
    buf_len: u32,
    mac_ptr: *mut u8,
    mac_len: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
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

// https://github.com/devkitPro/libogc/blob/master/gc/lwip/sockets.h#L38
// https://github.com/devkitPro/libogc/blob/dab81d801174e08b846cffe4531fe0325d1f5f8c/libogc/network_wii.c#L134
#[repr(C)]
#[derive(Clone, Copy)]
struct SocketConnectParams {
    socket:     u32,
    has_addr:   u32,
    sin_len:    u8,
    sin_family: u8,
    sin_port:   u16,
    sin_addr:   u32,
    sin_zero:   [u8; 20],
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct SocketSendToParams {
    socket:       u32,
    flags:        u32,
    has_destaddr: u32,
    destaddr:     [u8; 28],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SocketSendToIoctlv {
    msg_ptr:    *const u8,
    msg_len:    u32,
    params_ptr: *const u8,
    params_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SocketListenParams {
    socket:  u32,
    backlog: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct SocketAddrIn {
    sin_len:    u8,
    sin_family: u8,
    sin_port:   u16,
    sin_addr:   u32,
}

#[repr(C, align(0x20))]
union IoctlvUnion {
    none:           (),
    mac:            IoctlvMac,
    netstatus:      IoctlvLinkStatus,
    sock_init:      [c_int; 3],
    connect_params: SocketConnectParams,
    send_to_params: SocketSendToIoctlv,
    listen_params:  SocketListenParams,
    addr_in:        SocketAddrIn,
}

#[repr(C, align(0x20))]
pub struct NetInitData {
    ioctlv:           IoctlvUnion,
    pub manage_fd:    c_int,
    pub top_fd:       c_int,
    pub request_fd:   c_int,
    pub ip:           u32,
    pub udp_socket:   c_int,
    pub socket:       c_int,
    pub bound_socket: c_int,
    pub state:        usize,
    pub mac_buf:      [u8; 6],
    pub retry_alarm:  OSAlarm,
    pub last_err:     c_int,
    pub retry_count:  u32,
    ioctl_err:        c_int,
    cmd_buf:          AlignedBuf<0x20>,
    send_params:      SocketSendToParams,
}

impl Debug for NetInitData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NetInitDate")
            .field("manage_fd", &self.manage_fd)
            .field("top_fd", &self.top_fd)
            .field("request_fd", &self.request_fd)
            .field("ip", &self.ip)
            .field("state_id", &self.state)
            .field("state", &INIT_CHAIN[self.state])
            .field("socket", &self.socket)
            .field("bound_socket", &self.bound_socket)
            .field("mac_buf", &self.mac_buf)
            .field("last_err", &self.last_err)
            .field("ioctl_err", &self.ioctl_err)
            .field("retry_count", &self.retry_count)
            .finish()
    }
}

pub extern "C" fn net_init_retry_alarm_callback(alarm: *mut OSAlarm) {
    let usr_data =
        unsafe { field_offset::offset_of!(NetInitData => retry_alarm).unapply_ptr_mut(alarm) };
    let init_data = unsafe { &mut *usr_data };
    if let Some(next_state) = INIT_CHAIN.get(init_data.state) {
        let ret = next_state.do_ioctl(init_data, usr_data.cast());
        if ret != 0 {
            console_print(format_args!(
                "error with ioctl for {:?}: {}\n",
                next_state, ret
            ));
            init_data.ioctl_err = ret;
            do_retry_alarm(init_data);
        }
    }
}

fn retry_timeout() -> u64 {
    (get_time_base() * 4000).into()
}

#[link_section = ".data"]
#[no_mangle]
pub static mut INIT_CHAIN_DATA_PTR: *mut NetInitData = core::ptr::null_mut();

#[no_mangle]
pub fn do_init_chain() {
    if unsafe { !INIT_CHAIN_DATA_PTR.is_null() } {
        return;
    }
    let init_data = unsafe { ios_allocate::<NetInitData>() };
    init_data.write(NetInitData {
        ioctlv:       IoctlvUnion { none: () },
        manage_fd:    -1,
        top_fd:       -1,
        request_fd:   -1,
        ip:           0,
        udp_socket:   -1,
        socket:       -1,
        bound_socket: -1,
        state:        0,
        mac_buf:      [0; 6],
        last_err:     0,
        ioctl_err:    0,
        retry_count:  0,
        retry_alarm:  OSAlarm::new(),
        cmd_buf:      AlignedBuf::default(),
        send_params:  Default::default(),
    });
    unsafe { INIT_CHAIN_DATA_PTR = init_data.as_mut_ptr() };
    let init_data = unsafe { init_data.assume_init_mut() };
    if let Some(next_state) = INIT_CHAIN.get(init_data.state) {
        let usr_data = init_data as *mut NetInitData as *mut c_void;
        let ret = next_state.do_ioctl(init_data, usr_data);
        if ret != 0 {
            console_print(format_args!(
                "error with ioctl for {:?}: {}\n",
                next_state, ret
            ));
            init_data.ioctl_err = ret;
            do_retry_alarm(init_data);
        }
    }
}

fn do_retry_alarm(net_init: &mut NetInitData) {
    // retry after timeout
    unsafe {
        OSInsertAlarm(
            &mut net_init.retry_alarm as *mut OSAlarm,
            retry_timeout(),
            net_init_retry_alarm_callback,
        )
    };
}

#[no_mangle]
pub extern "C" fn net_init_callback(result: c_int, usr_data: *mut c_void) {
    let init_data = unsafe { &mut *(usr_data.cast::<NetInitData>()) };
    let current_state = INIT_CHAIN[init_data.state];
    console_print(format_args!(
        "getting called with {result} in {:?}\n",
        current_state
    ));
    if let Err(_) = current_state.on_result(init_data, result) {
        // error state, retry in a bit
        console_print(format_args!("{:?} failed: {result}\n", current_state));

        init_data.retry_count += 1;
        init_data.last_err = result;
        do_retry_alarm(init_data);
        return;
    }
    init_data.retry_count = 0;
    init_data.state += 1;
    if let Some(next_state) = INIT_CHAIN.get(init_data.state) {
        let ret = next_state.do_ioctl(init_data, usr_data);
        if ret != 0 {
            console_print(format_args!(
                "error with ioctl for {:?}: {}\n",
                next_state, ret
            ));
            init_data.ioctl_err = ret;
            do_retry_alarm(init_data);
            return;
        }
    } else {
        // init chain is done, now start the normal manager
        let top_fd = init_data.top_fd;
        let ip = Ipv4Addr::from(init_data.ip.to_be_bytes());
        let accept_socket = init_data.socket;
        // make sure it's not referenced
        let _ = init_data;
        unsafe { INIT_CHAIN_DATA_PTR = null_mut() };
        unsafe { ios_free(usr_data) };
        create_and_kickoff_net_manager(top_fd, accept_socket, ip);
    }
}

trait InitChainPart: Debug {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int;
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()>;
}

#[derive(Debug)]
struct OpenManageFd;
impl InitChainPart for OpenManageFd {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe {
            IOS_OpenAsync(
                cstr!("/dev/net/ncd/manage").as_ptr(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.manage_fd = result;
        Ok(())
    }
}

#[derive(Debug)]
struct GetLinkStatus;
impl InitChainPart for GetLinkStatus {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.netstatus.buf_len = 0x20;
        init_data.ioctlv.netstatus.buf_ptr = addr_of_mut!((*init_data).cmd_buf).cast();
        unsafe {
            IOS_IoctlvAsync(
                init_data.manage_fd,
                7, // IOCTL_NCD_GETLINKSTATUS
                0,
                1,
                usr_data,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CheckMac;
impl InitChainPart for CheckMac {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.mac.mac_len = 6;
        init_data.ioctlv.mac.mac_ptr = addr_of_mut!((*init_data).mac_buf).cast();
        init_data.ioctlv.mac.buf_len = 0x20;
        init_data.ioctlv.mac.buf_ptr = addr_of_mut!((*init_data).cmd_buf).cast();
        unsafe {
            IOS_IoctlvAsync(
                init_data.manage_fd,
                8, // MAC
                0,
                2,
                usr_data,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CloseManageFd;
impl InitChainPart for CloseManageFd {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe { IOS_CloseAsync(init_data.manage_fd, net_init_callback, usr_data) }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.manage_fd = -1;
        Ok(())
    }
}

#[derive(Debug)]
struct OpenTopFd;
impl InitChainPart for OpenTopFd {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe {
            IOS_OpenAsync(
                cstr!("/dev/net/ip/top").as_ptr(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.top_fd = result;
        Ok(())
    }
}

#[derive(Debug)]
struct OpenRequestFd;
impl InitChainPart for OpenRequestFd {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe {
            IOS_OpenAsync(
                cstr!("/dev/net/kd/request").as_ptr(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.request_fd = result;
        Ok(())
    }
}

#[derive(Debug)]
struct NWC24Startup;
impl InitChainPart for NWC24Startup {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe {
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
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        // -15 means already running
        if result < 0 && result != -15 {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CloseRequestFd;
impl InitChainPart for CloseRequestFd {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe { IOS_CloseAsync(init_data.request_fd, net_init_callback, usr_data) }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.request_fd = -1;
        Ok(())
    }
}

#[derive(Debug)]
struct SocketStartup;
impl InitChainPart for SocketStartup {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe {
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
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CheckIp;
impl InitChainPart for CheckIp {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        unsafe {
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
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result == 0 {
            return Err(());
        }
        init_data.ip = result as u32;
        Ok(())
    }
}

#[derive(Debug)]
struct CreateSocket;
impl InitChainPart for CreateSocket {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.sock_init = [
            2, // AF_INET
            1, // SOCK_STREAM
            0, // IPPROTO_IP
        ];
        unsafe {
            IOS_IoctlAsync(
                init_data.top_fd,
                15, // IOCTL_SO_SOCKET
                usr_data,
                12,
                null_mut(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.socket = result;
        Ok(())
    }
}

#[derive(Debug)]
struct CreateUdpSocket;
impl InitChainPart for CreateUdpSocket {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.sock_init = [
            2, // AF_INET
            2, // SOCK_DGRAM
            0, // IPPROTO_IP
        ];
        unsafe {
            IOS_IoctlAsync(
                init_data.top_fd,
                15, // IOCTL_SO_SOCKET
                usr_data,
                12,
                null_mut(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.udp_socket = result;
        Ok(())
    }
}

#[derive(Debug)]
struct SendUdpSocket;
impl InitChainPart for SendUdpSocket {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.sock_init = [
            2, // AF_INET
            2, // SOCK_DGRAM
            0, // IPPROTO_IP
        ];
        unsafe {
            IOS_IoctlAsync(
                init_data.top_fd,
                15, // IOCTL_SO_SOCKET
                usr_data,
                12,
                null_mut(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.udp_socket = result;
        Ok(())
    }
}

#[derive(Debug)]
struct ConnectSocket;
impl InitChainPart for ConnectSocket {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.connect_params = SocketConnectParams {
            socket:     init_data.socket as u32,
            has_addr:   1,
            sin_len:    8,
            sin_addr:   u32::from_be_bytes([192, 168, 0, 144]),
            sin_family: 2, // AF_INET
            sin_port:   43673,
            sin_zero:   Default::default(),
        };
        unsafe {
            IOS_IoctlAsync(
                init_data.top_fd,
                4, // IOCTL_SO_LISTEN
                usr_data,
                size_of::<SocketConnectParams>() as c_int,
                null_mut(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.bound_socket = result;
        Ok(())
    }
}

#[derive(Debug)]
struct SendSocketMessage;
impl InitChainPart for SendSocketMessage {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.cmd_buf.buf.fill(0);
        let msg = "hello\n\n";

        init_data.send_params = SocketSendToParams {
            has_destaddr: 1,
            flags:        0,
            socket:       init_data.udp_socket as u32,
            destaddr:     [
                8, 2, 0x30, 0x39, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0,
            ],
        };

        init_data.ioctlv.send_to_params = SocketSendToIoctlv {
            msg_len:    msg.len() as u32,
            msg_ptr:    msg.as_ptr().cast(),
            params_ptr: addr_of!((init_data.send_params)).cast(),
            params_len: size_of::<SocketSendToParams>() as u32,
        };

        unsafe {
            IOS_IoctlvAsync(
                init_data.top_fd,
                13,
                2,
                0,
                usr_data,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct BindSocket;
impl InitChainPart for BindSocket {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.connect_params = SocketConnectParams {
            socket:     init_data.socket as u32,
            has_addr:   1,
            sin_len:    8,
            sin_family: 2, // AF_INET
            sin_port:   43673,
            sin_addr:   init_data.ip,
            sin_zero:   Default::default(),
        };
        unsafe {
            IOS_IoctlAsync(
                init_data.top_fd,
                2, // IOCTL_SO_BIND
                addr_of_mut!(init_data.ioctlv.connect_params).cast(),
                size_of::<SocketConnectParams>() as c_int,
                null_mut(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ListenSocket;
impl InitChainPart for ListenSocket {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        init_data.ioctlv.listen_params = SocketListenParams {
            socket:  init_data.socket as u32,
            backlog: 0,
        };
        unsafe {
            IOS_IoctlAsync(
                init_data.top_fd,
                10, // IOCTL_SO_LISTEN
                addr_of_mut!(init_data.ioctlv.listen_params).cast(),
                size_of::<SocketListenParams>() as c_int,
                null_mut(),
                0,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct AcceptSocket;
impl InitChainPart for AcceptSocket {
    fn do_ioctl(&self, init_data: &mut NetInitData, usr_data: *mut c_void) -> c_int {
        console_print(format_args!("now accepting\n"));
        unsafe {
            IOS_IoctlAsync(
                init_data.top_fd,
                1, // IOCTL_SO_ACCEPT
                // only a single parameter
                addr_of_mut!(init_data.socket).cast(),
                size_of::<u32>() as c_int,
                addr_of_mut!(init_data.ioctlv.addr_in).cast(),
                size_of::<SocketAddrIn>() as c_int,
                net_init_callback,
                usr_data,
            )
        }
    }
    fn on_result(&self, init_data: &mut NetInitData, result: c_int) -> Result<(), ()> {
        if result < 0 {
            return Err(());
        }
        init_data.bound_socket = result;
        let addr = unsafe { &init_data.ioctlv.addr_in };
        console_print(format_args!("address: {addr:?}\n"));
        console_print(format_args!("old sock: {}\n", init_data.socket));
        Ok(())
    }
}

static INIT_CHAIN: &[&(dyn InitChainPart + Sync)] = &[
    &OpenManageFd,
    &GetLinkStatus,
    &CheckMac,
    &CloseManageFd,
    &OpenTopFd,
    &OpenRequestFd,
    &NWC24Startup, // needed!
    &CloseRequestFd,
    &SocketStartup,
    &CheckIp,
    &CreateUdpSocket,
    &SendSocketMessage,
    &CreateSocket,
    &BindSocket,
    &ListenSocket,
    // &ConnectSocket,
    // &AcceptSocket,
    // &SendSocketMessage,
];

#[derive(Debug, Default)]
#[repr(C, align(0x20))]
struct RecvParams {
    socket: c_int,
    flags:  c_int,
}

#[repr(C, align(0x20))]
struct RecvIoctl {
    params_ptr: *const c_void,
    params_len: u32,
    buffer_ptr: *mut c_char,
    buffer_len: u32,
    addr_ptr:   *mut c_void,
    addr_len:   u32,
}

#[repr(C, align(0x20))]
struct SocketShutdownParams {
    socket: c_int,
    how:    c_int,
}

pub struct NetManager {
    top_fd:          c_int,
    accept_socket:   c_int,
    current_socket:  Option<c_int>,
    ip:              Ipv4Addr,
    recv_buf:        AlignedBuf<1024>,
    recv_ioctl:      RecvIoctl,
    recv_params:     RecvParams,
    recv_manager:    RecvManager,
    accept_addr:     SocketAddrIn,
    shutdown_params: SocketShutdownParams,
    is_accept:       bool,
    is_recv:         bool,
    is_close:        bool,
    last_err:        c_int,
}

impl Debug for NetManager {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NetManager")
            .field("top_fd", &self.top_fd)
            .field("accept_socket", &self.accept_socket)
            .field("current_socket", &self.current_socket)
            .field("ip", &self.ip)
            .field("is_accept", &self.is_accept)
            .field("is_recv", &self.is_recv)
            .field("is_close", &self.is_close)
            .field("last_err", &self.last_err)
            .finish()
    }
}

impl NetManager {
    pub fn try_from_static<'a>() -> Option<&'a NetManager> {
        unsafe { (!NET_MANAGER.is_null()).then(|| &*NET_MANAGER) }
    }
    fn from_usr_data<'a>(usr_data: *mut c_void) -> &'a mut NetManager {
        unsafe { &mut *(usr_data.cast()) }
    }
    fn to_usr_data(&mut self) -> *mut c_void {
        self as *mut NetManager as *mut c_void
    }
}

struct RecvManager;
impl RecvManager {
    pub fn process(&mut self, data: &[u8]) {
        if let Ok(s) = from_utf8(data) {
            console_print(format_args!("got: {s}\n"));
        } else {
            console_print(format_args!("got bytes: {data:?}\n"));
        }
    }
}

struct SendManager;
impl SendManager {
    pub fn advance(&mut self, bytes: usize) {
        // todo
    }
    pub fn next_chunk(&mut self) -> Option<NonNull<[u8]>> {
        None
    }
}

fn do_recv(mgr: &mut NetManager) {
    let Some(cur_socket) = mgr.current_socket else {
        return;
    };
    mgr.recv_params = RecvParams {
        flags:  0,
        socket: cur_socket as c_int,
    };
    mgr.recv_ioctl = RecvIoctl {
        params_ptr: addr_of!(mgr.recv_params).cast(),
        params_len: size_of::<RecvParams>() as u32,
        buffer_ptr: addr_of_mut!(mgr.recv_buf).cast(),
        buffer_len: size_of_val(&mgr.recv_buf) as u32,
        addr_ptr:   null_mut(),
        addr_len:   0,
    };
    mgr.is_recv = true;
    unsafe {
        IOS_IoctlvAsync(
            mgr.top_fd as i32,
            12, // IOCTLV_SO_RECVFROM
            1,
            2,
            addr_of_mut!(mgr.recv_ioctl).cast(),
            on_recv,
            mgr.to_usr_data(),
        )
    };
}

#[no_mangle]
extern "C" fn on_recv(result: c_int, usr_data: *mut c_void) {
    console_print(format_args!("on recv: {result}"));
    let mgr = NetManager::from_usr_data(usr_data);
    mgr.is_recv = false;
    if result == -8 {
        // E_BADFD, topfd closed
        return;
    }
    if result < 0 {
        console_print(format_args!("got error on recv: {result}"));
        // shutdown (probably)
        return;
    } else if result == 0 {
        // socket is closed
        do_close_current(mgr);
        return;
    }
    let result_data = &mgr.recv_buf.buf[..result as usize];

    if result_data.starts_with(b"pos") && !player::get_ptr().is_null() {
        let mut buf = arrayvec::ArrayString::<100>::new();
        let (x, y, z) = unsafe {
            (
                (*player::get_ptr()).pos_x,
                (*player::get_ptr()).pos_y,
                (*player::get_ptr()).pos_z,
            )
        };
        let _ = writeln!(&mut buf, "{x}:{y}:{z}");
        send_write_request(mgr, buf.as_bytes());
    }

    if result_data.starts_with(b"fly") && unsafe { !player::get_ptr().is_null() } {
        unsafe { (*player::get_ptr()).pos_y += 200f32 };
        send_write_request(mgr, b"weeeeeeee\n");
    }

    // mgr.recv_manager.process(&mgr.recv_buf.buf[..result]);
    do_recv(mgr);
}

fn do_accept(mgr: &mut NetManager) {
    mgr.is_accept = true;
    unsafe {
        IOS_IoctlAsync(
            mgr.top_fd as c_int,
            1, // IOCTL_SO_ACCEPT
            // only a single parameter
            addr_of_mut!(mgr.accept_socket).cast(),
            size_of_val(&mgr.accept_socket) as c_int,
            addr_of_mut!(mgr.accept_addr).cast(),
            size_of_val(&mgr.accept_addr) as c_int,
            on_accept,
            mgr.to_usr_data(),
        )
    };
}

// Try a UDP broadcast, maybe it can be found then?

#[no_mangle]
extern "C" fn on_accept(result: c_int, usr_data: *mut c_void) {
    let mgr = NetManager::from_usr_data(usr_data);
    mgr.is_accept = false;
    if result == -8 {
        // E_BADFD, topfd closed
        return;
    }
    if result < 0 {
        mgr.last_err = result;
        console_print(format_args!("got error on accept: {result}"));
        // don't try again, the socket got probably closed due to shutdown
        return;
    }
    mgr.current_socket = Some(result);
    // we now wait for data
    do_recv(mgr);
}

fn do_close_current(mgr: &mut NetManager) {
    let Some(cur_socket) = mgr.current_socket else {
        return;
    };
    mgr.is_close = true;
    mgr.shutdown_params = SocketShutdownParams {
        socket: cur_socket,
        how:    2, // close read & write
    };
    unsafe {
        IOS_IoctlAsync(
            mgr.top_fd as c_int,
            14, // IOCTL_SO_SHUTDOWN
            addr_of_mut!(mgr.shutdown_params).cast(),
            size_of_val(&mgr.shutdown_params) as c_int,
            null_mut(),
            0,
            on_close,
            mgr.to_usr_data(),
        )
    };
}

#[no_mangle]
extern "C" fn on_close(result: c_int, usr_data: *mut c_void) {
    // we don't actually care if it was successful
    console_print(format_args!("closed socket with result: {result}"));
    let mgr = NetManager::from_usr_data(usr_data);
    do_accept(mgr);
}

fn create_and_kickoff_net_manager(top_fd: c_int, accept_socket: c_int, ip: Ipv4Addr) {
    console_print(format_args!(
        "net manager:\ntop: {top_fd}\nsocket: {accept_socket}\nip:{ip}\n"
    ));
    let uninit_mgr = unsafe { ios_allocate::<NetManager>() };
    uninit_mgr.write(NetManager {
        accept_addr: SocketAddrIn::default(),
        accept_socket,
        current_socket: None,
        ip,
        recv_buf: Default::default(),
        recv_ioctl: RecvIoctl {
            params_ptr: null(),
            params_len: 0,
            buffer_ptr: null_mut(),
            buffer_len: 0,
            addr_ptr:   null_mut(),
            addr_len:   0,
        },
        recv_manager: RecvManager,
        recv_params: Default::default(),
        top_fd,
        is_accept: false,
        is_close: false,
        is_recv: false,
        shutdown_params: SocketShutdownParams {
            socket: 0,
            how:    0,
        },
        last_err: 0,
    });
    let mgr = unsafe { uninit_mgr.assume_init_mut() };
    unsafe { NET_MANAGER = mgr as *mut NetManager };
    do_accept(mgr);
}

#[no_mangle]
pub extern "C" fn net_mgr_shutdown(ret: c_int) -> c_int {
    if unsafe { !NET_MANAGER.is_null() } {
        let shutdown_params = unsafe { ios_allocate::<SocketShutdownParams>().assume_init_mut() };
        shutdown_params.socket = unsafe { (*NET_MANAGER).accept_socket };
        shutdown_params.how = 2;
        let top_fd = unsafe { (*NET_MANAGER).top_fd };
        unsafe {
            IOS_Close(top_fd);
        }
    }
    ret
}

#[link_section = ".data"]
#[no_mangle]
static mut NET_MANAGER: *mut NetManager = null_mut();

#[repr(C, align(0x20))]
struct SendReqHead {
    top_fd:      c_int,
    send_ioctl:  SocketSendToIoctlv,
    send_params: SocketSendToParams,
}

fn send_write_request(mgr: &NetManager, data: &[u8]) {
    let Some(cur_socket) = mgr.current_socket else {
        return;
    };
    unsafe {
        let data_ptr = iosAllocAligned(IOS_HEAP, data.len() as u32, 0x20);
        copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
        let req = &mut *(iosAllocAligned(
            IOS_HEAP,
            size_of::<SendReqHead>() as u32,
            align_of::<SendReqHead>() as i32,
        ) as *mut SendReqHead);
        req.top_fd = mgr.top_fd;
        req.send_params = SocketSendToParams {
            socket: cur_socket as u32,
            has_destaddr: 0,
            flags: 0,
            ..Default::default()
        };
        req.send_ioctl = SocketSendToIoctlv {
            msg_len:    data.len() as u32,
            msg_ptr:    data_ptr,
            params_len: size_of_val(&req.send_params) as u32,
            params_ptr: addr_of_mut!(req.send_params).cast(),
        };

        IOS_IoctlvAsync(
            mgr.top_fd as c_int,
            13, // send
            2,
            0,
            addr_of_mut!(req.send_ioctl).cast(),
            on_write_return,
            req as *mut SendReqHead as *mut c_void,
        );
    }
}

#[no_mangle]
extern "C" fn on_write_return(result: c_int, usr_data: *mut c_void) {
    // if it wasn't successful, just throw it out for now
    let req = usr_data as *mut SendReqHead;
    unsafe {
        console_print(format_args!(
            "wrote {} from {}\n",
            result,
            (*req).send_ioctl.msg_len
        ));
        iosFree(IOS_HEAP, (*req).send_ioctl.msg_ptr.cast_mut());
        iosFree(IOS_HEAP, req.cast());
    }
}
