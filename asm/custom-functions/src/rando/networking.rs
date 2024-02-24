use core::{
    borrow::BorrowMut,
    cell::Cell,
    ffi::{c_int, c_void, CStr},
    future::Future,
    pin::Pin,
    ptr::null_mut,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use alloc::boxed::Box;
use cstr::cstr;

use crate::{
    console_print,
    rvl_mem::IosAllocator,
    system::{
        alarm::{OSAlarm, OSInsertAlarm},
        ios::{IOS_CloseAsync, IOS_IoctlAsync, IOS_IoctlvAsync, IOS_OpenAsync},
        time::get_time_base,
    },
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

    pub fn do_poll(&mut self) {
        let cx = RawWaker::new(
            self as *const IosAsyncContext as *const (),
            Waker::noop().as_raw().vtable(),
        );
        let _ =
            unsafe { Pin::new(&mut self.fut).poll(&mut Context::from_waker(&Waker::from_raw(cx))) };
    }
}

#[no_mangle]
extern "C" fn post_ios(result: c_int, usr_data: *mut c_void) {
    let ios_ctx = unsafe { &mut *(usr_data as *mut IosAsyncContext) };
    ios_ctx.result.set(Some(result));
    ios_ctx.do_poll();
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
    ios_ctx.do_poll();
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
    ios_ctx.do_poll();
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

async fn net_init_stuff() {
    if let Err(e) = try_net_init_stuff().await {
        console_print(format_args!("net init err: {e}\n"));
    }
}

async fn try_net_init_stuff() -> Result<(), i32> {
    let manage_fd = ios_open(cstr!("/dev/net/ncd/manage")).await?;
    let top_fd = ios_open(cstr!("/dev/net/ip/top")).await?;
    let request_fd = ios_open(cstr!("/dev/net/kd/request")).await?;
    for _ in 0..3 {
        console_print(format_args!("{manage_fd} {top_fd} {request_fd}\n"));
        sleep(get_time_base() as u64 * 4000).await;
    }
    Ok(())
}
