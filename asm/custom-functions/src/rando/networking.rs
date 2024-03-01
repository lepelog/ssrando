use core::{
    borrow::BorrowMut,
    cell::Cell,
    ffi::{c_int, c_uint, c_ushort, c_void, CStr},
    fmt::Debug,
    future::Future,
    mem::size_of_val,
    net::Ipv4Addr,
    pin::Pin,
    ptr::null_mut,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use alloc::boxed::Box;
use cstr::cstr;

use crate::{
    console_print, print_cstr,
    rvl_mem::IosAllocator,
    system::{
        alarm::{OSAlarm, OSInsertAlarm},
        ios::{IOS_CloseAsync, IOS_IoctlAsync, IOS_IoctlvAsync, IOS_OpenAsync},
        time::get_time_base,
    },
    utils::AlignedBuf,
};

pub struct IosAsyncContext {
    // if this is None, no async operation was in progress
    // otherwise contains the result
    pub result: Cell<Option<i32>>,
    pub fut:    Pin<Box<dyn Future<Output = ()>, IosAllocator>>,
}

impl IosAsyncContext {
    pub fn from_ctx<'a>(cx: &'a mut Context<'_>) -> &'a IosAsyncContext {
        unsafe { &*(cx.waker().as_raw().data() as *const IosAsyncContext) }
    }

    pub fn from_ptr<'a>(usr_data: *mut c_void) -> &'a IosAsyncContext {
        unsafe { &*(usr_data as *const IosAsyncContext) }
    }

    pub fn do_poll(this: *mut IosAsyncContext) {
        let cx = RawWaker::new(this as *const (), Waker::noop().as_raw().vtable());
        if let Poll::Ready(()) = unsafe {
            Pin::new(&mut (*this).fut).poll(&mut Context::from_waker(&Waker::from_raw(cx)))
        } {
            // feature is done, destruct it
            unsafe { Box::from_raw_in(this, IosAllocator) };
        }
    }
}

#[no_mangle]
extern "C" fn post_ios(result: c_int, usr_data: *mut c_void) {
    let ios_ctx = unsafe { &mut *(usr_data as *mut IosAsyncContext) };
    ios_ctx.result.set(Some(result));
    IosAsyncContext::do_poll(ios_ctx as *mut _);
}

#[no_mangle]
pub extern "C" fn run_net_init() {
    let net_fut = Box::pin_in(net_init_stuff(), IosAllocator);

    let ios_ctx = Box::new_in(
        IosAsyncContext {
            fut:    net_fut,
            result: Cell::new(None),
        },
        IosAllocator,
    );
    let ios_ctx = Box::leak(ios_ctx);
    IosAsyncContext::do_poll(ios_ctx as *mut _);
}

pub struct IosOpenFut<'a> {
    path: &'a CStr,
}

impl<'a> Future for IosOpenFut<'a> {
    type Output = Result<i32, i32>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = IosAsyncContext::from_ctx(cx).result.take() {
            if result < 0 {
                Poll::Ready(Err(result))
            } else {
                Poll::Ready(Ok(result))
            }
        } else {
            let result = unsafe {
                IOS_OpenAsync(
                    self.path.as_ptr(),
                    0,
                    post_ios,
                    cx.waker().as_raw().data() as *mut c_void,
                )
            };
            if result != 0 {
                Poll::Ready(Err(result))
            } else {
                Poll::Pending
            }
        }
    }
}

fn ios_open<'a>(path: &'a CStr) -> impl Future<Output = Result<i32, i32>> + 'a {
    IosOpenFut { path }
}

pub struct IosCloseFut {
    fd: c_int,
}

impl Future for IosCloseFut {
    type Output = Result<i32, i32>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = IosAsyncContext::from_ctx(cx).result.take() {
            if result < 0 {
                Poll::Ready(Err(result))
            } else {
                Poll::Ready(Ok(result))
            }
        } else {
            let result = unsafe {
                IOS_CloseAsync(self.fd, post_ios, cx.waker().as_raw().data() as *mut c_void)
            };
            if result != 0 {
                Poll::Ready(Err(result))
            } else {
                Poll::Pending
            }
        }
    }
}

fn ios_close<'a>(fd: c_int) -> impl Future<Output = Result<i32, i32>> + 'a {
    IosCloseFut { fd }
}

pub struct IosIoctlvFut {
    fd:      c_int,
    command: c_int,
    in_cnt:  c_int,
    out_cnt: c_int,
    ioctlv:  *mut c_void,
}

impl Future for IosIoctlvFut {
    type Output = Result<i32, i32>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = IosAsyncContext::from_ctx(cx).result.take() {
            if result < 0 {
                Poll::Ready(Err(result))
            } else {
                Poll::Ready(Ok(result))
            }
        } else {
            let result = unsafe {
                IOS_IoctlvAsync(
                    self.fd,
                    self.command,
                    self.in_cnt,
                    self.out_cnt,
                    self.ioctlv,
                    post_ios,
                    cx.waker().as_raw().data() as *mut c_void,
                )
            };
            if result != 0 {
                Poll::Ready(Err(result))
            } else {
                Poll::Pending
            }
        }
    }
}

pub struct IosIoctlFut {
    fd:      c_int,
    command: c_int,
    in_buf:  *mut c_void,
    in_len:  c_int,
    out_buf: *mut c_void,
    out_len: c_int,
}

impl Future for IosIoctlFut {
    type Output = Result<i32, i32>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = IosAsyncContext::from_ctx(cx).result.take() {
            // some commands have a non error response that is negative
            Poll::Ready(Ok(result))
        } else {
            let result = unsafe {
                IOS_IoctlAsync(
                    self.fd,
                    self.command,
                    self.in_buf,
                    self.in_len,
                    self.out_buf,
                    self.out_len,
                    post_ios,
                    cx.waker().as_raw().data() as *mut c_void,
                )
            };
            if result != 0 {
                Poll::Ready(Err(result))
            } else {
                Poll::Pending
            }
        }
    }
}

struct OSAlarmWithIosAsyncContext {
    alarm:   OSAlarm,
    context: *mut IosAsyncContext,
}

struct AlarmFut<'a> {
    os_alarm: &'a mut OSAlarmWithIosAsyncContext,
    timeout:  u64,
}

extern "C" fn alarm_callback(alarm: *mut OSAlarm) {
    let ios_ctx = unsafe { &mut *(*(alarm as *mut OSAlarmWithIosAsyncContext)).context };
    ios_ctx.result.set(Some(0));
    IosAsyncContext::do_poll(ios_ctx as *mut _);
}

impl<'a> Future for AlarmFut<'a> {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(_) = IosAsyncContext::from_ctx(cx).result.take() {
            Poll::Ready(())
        } else {
            self.os_alarm.context = IosAsyncContext::from_ctx(cx) as *const _ as *mut _;
            unsafe {
                OSInsertAlarm(
                    self.os_alarm.borrow_mut() as *mut OSAlarmWithIosAsyncContext as *mut _,
                    self.timeout,
                    alarm_callback,
                )
            };
            Poll::Pending
        }
    }
}

async fn sleep(timeout: u64) {
    let mut alarm = OSAlarmWithIosAsyncContext {
        alarm:   OSAlarm::new(),
        context: null_mut(),
    };
    AlarmFut {
        os_alarm: &mut alarm,
        timeout,
    }
    .await
}

fn map_standard_result(result: Result<i32, i32>) -> Result<i32, i32> {
    if let Ok(value) = result {
        if value < 0 {
            return Err(value);
        }
    }
    result
}

#[repr(C, align(0x20))]
#[derive(Clone, Copy)]
struct SocketConnectParams {
    socket:     c_int,
    has_addr:   u32,
    sin_len:    u8,
    sin_family: u8,
    sin_port:   u16,
    sin_addr:   u32,
    sin_zero:   [u8; 20],
}

#[repr(C, align(0x20))]
#[derive(Default, Debug, Clone, Copy)]
struct SocketAddrIn {
    sin_len:    u8,
    sin_family: u8,
    sin_port:   u16,
    sin_addr:   u32,
}

struct ManageFd {
    fd: c_int,
}

impl ManageFd {
    async fn open() -> Result<Self, c_int> {
        ios_open(cstr!("/dev/net/ncd/manage"))
            .await
            .map(|fd| Self { fd })
    }

    async fn close(self) {
        // closing should really not fail
        let _ = ios_close(self.fd).await;
    }
}

struct TopFd {
    fd: c_int,
}

impl TopFd {
    async fn open() -> Result<Self, c_int> {
        ios_open(cstr!("/dev/net/ip/top"))
            .await
            .map(|fd| Self { fd })
    }

    async fn close(self) {
        // closing should really not fail
        let _ = ios_close(self.fd).await;
    }

    async fn socket_startup(&self) -> Result<i32, i32> {
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 31, // IOCTL_SO_STARTUP
            in_buf:  null_mut(),
            in_len:  0,
            out_buf: null_mut(),
            out_len: 0,
        }
        .await;
        map_standard_result(result)
    }

    async fn get_host_id(&self) -> Result<i32, i32> {
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 16, // IOCTL_SO_GETHOSTID
            in_buf:  null_mut(),
            in_len:  0,
            out_buf: null_mut(),
            out_len: 0,
        }
        .await;
        if matches!(result, Ok(0)) {
            return Err(0);
        }
        result
    }

    async fn create_tcp_socket(&self) -> Result<i32, i32> {
        let mut sock_init: AlignedBuf<3, i32> = AlignedBuf {
            buf: [
                2, // AF_INET
                1, // SOCK_STREAM
                0, // IPPROTO_IP
            ],
        };
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 15, // IOCTL_SO_SOCKET
            in_buf:  sock_init.as_mut_ptr() as *mut _,
            in_len:  size_of_val(&sock_init) as i32,
            out_buf: null_mut(),
            out_len: 0,
        }
        .await;
        map_standard_result(result)
    }

    async fn create_udp_socket(&self) -> Result<i32, i32> {
        let mut sock_init: AlignedBuf<3, i32> = AlignedBuf {
            buf: [
                2, // AF_INET
                2, // SOCK_DGRAM
                0, // IPPROTO_IP
            ],
        };
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 15, // IOCTL_SO_SOCKET
            in_buf:  sock_init.as_mut_ptr() as *mut _,
            in_len:  size_of_val(&sock_init) as i32,
            out_buf: null_mut(),
            out_len: 0,
        }
        .await;
        map_standard_result(result)
    }

    async fn bind_socket(&self, socket: c_int, addr: c_uint, port: c_ushort) -> Result<i32, i32> {
        let mut params = SocketConnectParams {
            socket,
            has_addr: 1,
            sin_len: 8,
            sin_family: 2, // AF_INET
            sin_port: port,
            sin_addr: addr,
            sin_zero: Default::default(),
        };
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 2, // IOCTL_SO_BIND
            in_buf:  &mut params as *mut SocketConnectParams as *mut _,
            in_len:  size_of_val(&params) as i32,
            out_buf: null_mut(),
            out_len: 0,
        }
        .await;
        map_standard_result(result)
    }

    async fn connect_socket(
        &self,
        socket: c_int,
        addr: c_uint,
        port: c_ushort,
    ) -> Result<i32, i32> {
        let mut params = SocketConnectParams {
            socket,
            has_addr: 1,
            sin_len: 8,
            sin_family: 2, // AF_INET
            sin_port: port,
            sin_addr: addr,
            sin_zero: Default::default(),
        };
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 4, // IOCTL_SO_CONNECT
            in_buf:  &mut params as *mut SocketConnectParams as *mut _,
            in_len:  size_of_val(&params) as i32,
            out_buf: null_mut(),
            out_len: 0,
        }
        .await;
        map_standard_result(result)
    }

    async fn listen_socket(&self, socket: c_int, backlog: c_uint) -> Result<i32, i32> {
        let mut params = AlignedBuf {
            buf: [socket as u32, backlog as u32],
        };
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 10, // IOCTL_SO_LISTEN
            in_buf:  params.as_mut_ptr() as *mut _,
            in_len:  size_of_val(&params) as i32,
            out_buf: null_mut(),
            out_len: 0,
        }
        .await;
        map_standard_result(result)
    }

    async fn accept_socket(&self, socket: c_int) -> Result<(i32, SocketAddrIn), i32> {
        let mut params = AlignedBuf {
            buf: [socket as u32],
        };
        let mut addr_out = SocketAddrIn::default();
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 1, // IOCTL_SO_ACCEPT
            in_buf:  params.as_mut_ptr() as *mut _,
            in_len:  size_of_val(&params) as i32,
            out_buf: &mut addr_out as *mut SocketAddrIn as *mut _,
            out_len: size_of_val(&addr_out) as i32,
        }
        .await;
        match result {
            Ok(val) => {
                if val < 0 {
                    Err(val)
                } else {
                    Ok((val, addr_out))
                }
            },
            Err(e) => Err(e),
        }
    }

    async fn send_message(
        &self,
        socket: c_int,
        message: &[u8],
        destaddr: Option<IpV4DestAddr>,
    ) -> Result<i32, i32> {
        #[repr(C, align(0x20))]
        #[derive(Default, Debug, Clone, Copy)]
        struct SocketSendToParams {
            socket:       c_int,
            flags:        u32,
            has_destaddr: u32,
            destaddr:     [u8; 8],
        }
        let params = SocketSendToParams {
            socket,
            flags: 0,
            has_destaddr: destaddr.is_some().into(),
            destaddr: destaddr.map(|a| a.to_array()).unwrap_or_default(),
        };
        let mut ioctlv = AlignedBuf {
            buf: [
                message.as_ptr() as u32,
                message.len() as u32,
                &params as *const SocketSendToParams as u32,
                size_of_val(&params) as u32,
            ],
        };
        let result = IosIoctlvFut {
            fd:      self.fd,
            command: 13, // IOCTL_SO_SEND
            in_cnt:  2,
            out_cnt: 0,
            ioctlv:  ioctlv.as_mut_ptr() as *mut _,
        }
        .await;
        map_standard_result(result)
    }
}

struct IpV4DestAddr {
    ip:   u32,
    port: u16,
}

impl IpV4DestAddr {
    fn to_array(&self) -> [u8; 8] {
        let port_b = self.port.to_be_bytes();
        let ip_b = self.ip.to_be_bytes();
        [
            8, 2, port_b[0], port_b[1], ip_b[0], ip_b[1], ip_b[2], ip_b[3],
        ]
    }
}

struct RequestFd {
    fd: c_int,
}

impl RequestFd {
    async fn open() -> Result<Self, c_int> {
        ios_open(cstr!("/dev/net/kd/request"))
            .await
            .map(|fd| Self { fd })
    }

    async fn close(self) {
        // closing should really not fail
        let _ = ios_close(self.fd).await;
    }

    async fn nwc_24_startup(&self) -> Result<i32, i32> {
        let mut cmd_buf: AlignedBuf<0x20> = AlignedBuf::default();
        let result = IosIoctlFut {
            fd:      self.fd,
            command: 6, // IOCTL_NWC24_STARTUP
            in_buf:  null_mut(),
            in_len:  0,
            out_buf: cmd_buf.as_mut_ptr() as *mut c_void,
            out_len: cmd_buf.len() as i32,
        }
        .await;
        map_standard_result(result)
    }
}

async fn net_init_stuff() {
    if let Err(e) = try_net_init_stuff().await {
        console_print(format_args!("net init err: {e}\n"));
    }
}

async fn try_net_init_stuff() -> Result<(), i32> {
    let request_fd = RequestFd::open().await?;
    print_cstr(cstr!("req open\n"));
    let _ = request_fd.nwc_24_startup().await;
    request_fd.close().await;
    print_cstr(cstr!("nwc24\n"));
    let top_fd = TopFd::open().await?;
    top_fd.socket_startup().await?;
    print_cstr(cstr!("sock start\n"));
    let ip = top_fd.get_host_id().await?;
    let ip = Ipv4Addr::from(ip as u32);
    console_print(format_args!("ip: {}\n", ip));
    let sock = top_fd.create_tcp_socket().await?;
    print_cstr(cstr!("sock create\n"));
    top_fd
        .connect_socket(sock, u32::from_be_bytes([192, 168, 0, 144]), 43673)
        .await?;
    print_cstr(cstr!("sock connect\n"));
    let message = "hello, world!\n";
    loop {
        if let Err(e) = top_fd.send_message(sock, message.as_bytes(), None).await {
            console_print(format_args!("send err: {e}\n"));
        } else {
            break;
        }
    }
    console_print(format_args!("{} {sock}\n", top_fd.fd));
    Ok(())
}
