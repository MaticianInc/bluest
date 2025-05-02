use std::ffi::c_void;

use core_foundation_sys::{
    base::{Boolean, CFOptionFlags},
    runloop::CFRunLoopRef,
    stream::{
        CFReadStreamClose, CFReadStreamOpen, CFReadStreamRef, CFReadStreamScheduleWithRunLoop, CFReadStreamSetClient,
        CFReadStreamUnscheduleFromRunLoop, CFStreamClientContext, CFStreamEventType, CFWriteStreamClose,
        CFWriteStreamOpen, CFWriteStreamRef, CFWriteStreamScheduleWithRunLoop, CFWriteStreamSetClient,
        CFWriteStreamUnscheduleFromRunLoop,
    },
    string::CFStringRef,
};

type ClientCallBack<T> = extern "C" fn(stream: T, _type: CFStreamEventType, client_callback_info: *mut c_void);

/// Generic functions for both CFReadStream and CFWriteStream
pub trait CFStreamSchedule {
    unsafe fn schedule(&self, run_loop: CFRunLoopRef, mode: CFStringRef);
    unsafe fn unsubscribe(&self, run_loop: CFRunLoopRef, mode: CFStringRef);
    unsafe fn open(&self) -> Boolean;
    unsafe fn close(&self);
    unsafe fn set_client(
        &self,
        stream_events: CFOptionFlags,
        client_callback: ClientCallBack<Self>,
        context: *mut CFStreamClientContext,
    ) -> Boolean;
}

impl CFStreamSchedule for CFReadStreamRef {
    unsafe fn schedule(&self, run_loop: CFRunLoopRef, mode: CFStringRef) {
        unsafe { CFReadStreamScheduleWithRunLoop(*self, run_loop, mode) }
    }

    unsafe fn unsubscribe(&self, run_loop: CFRunLoopRef, mode: CFStringRef) {
        unsafe { CFReadStreamUnscheduleFromRunLoop(*self, run_loop, mode) }
    }

    unsafe fn open(&self) -> Boolean {
        CFReadStreamOpen(*self)
    }

    unsafe fn close(&self) {
        CFReadStreamClose(*self)
    }

    unsafe fn set_client(
        &self,
        stream_events: CFOptionFlags,
        client_callback: ClientCallBack<Self>,
        context: *mut CFStreamClientContext,
    ) -> Boolean {
        unsafe { CFReadStreamSetClient(*self, stream_events, client_callback, context) }
    }
}

impl CFStreamSchedule for CFWriteStreamRef {
    unsafe fn schedule(&self, run_loop: CFRunLoopRef, mode: CFStringRef) {
        unsafe { CFWriteStreamScheduleWithRunLoop(*self, run_loop, mode) }
    }

    unsafe fn unsubscribe(&self, run_loop: CFRunLoopRef, mode: CFStringRef) {
        unsafe { CFWriteStreamUnscheduleFromRunLoop(*self, run_loop, mode) }
    }

    unsafe fn open(&self) -> Boolean {
        CFWriteStreamOpen(*self)
    }

    unsafe fn close(&self) {
        CFWriteStreamClose(*self)
    }

    unsafe fn set_client(
        &self,
        stream_events: CFOptionFlags,
        client_callback: ClientCallBack<Self>,
        context: *mut CFStreamClientContext,
    ) -> Boolean {
        unsafe { CFWriteStreamSetClient(*self, stream_events, client_callback, context) }
    }
}

pub type CFReadStream = CFStream<CFReadStreamRef>;
pub type CFWriteStream = CFStream<CFWriteStreamRef>;

pub struct CFStream<T: CFStreamSchedule> {
    stream: T,
    scheduled: Option<(CFRunLoopRef, CFStringRef)>,
    open: bool,
}

impl<T: CFStreamSchedule> CFStream<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream: stream,
            scheduled: None,
            open: false,
        }
    }

    pub unsafe fn schedule(&mut self, run_loop: CFRunLoopRef, mode: CFStringRef) {
        unsafe { self.stream.schedule(run_loop, mode) };
        self.scheduled = Some((run_loop, mode));
    }

    pub unsafe fn open(&mut self) -> bool {
        match self.stream.open() {
            1 => {
                self.open = true;
                true
            }
            0 => false,
            _ => unreachable!("*StreamOpen returns a bool"),
        }
    }

    pub unsafe fn set_client(
        &self,
        stream_events: CFOptionFlags,
        client_callback: ClientCallBack<T>,
        context: *mut CFStreamClientContext,
    ) -> Boolean {
        unsafe { self.stream.set_client(stream_events, client_callback, context) }
    }
}

impl<T: CFStreamSchedule> Drop for CFStream<T> {
    fn drop(&mut self) {
        if self.open {
            unsafe { self.stream.close() };
        }
        if let Some((run_loop, mode)) = self.scheduled {
            unsafe { self.stream.unsubscribe(run_loop, mode) };
        }
    }
}
