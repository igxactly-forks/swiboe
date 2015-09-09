// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![feature(cstr_memory)]
#![feature(result_expect)]

extern crate libc;
extern crate serde;
extern crate serde_json;
extern crate swiboe;

use libc::c_char;
use std::ffi::{CStr, CString};
use std::mem;
use std::path;
use std::str;
use swiboe::client;
use swiboe::rpc;

// TODO(sirver): this always makes a copy, even though it might not be needed.
fn c_str_to_string(c_buf: *const c_char) -> String {
    let c_str = unsafe { CStr::from_ptr(c_buf) };
    let buf: &[u8] = c_str.to_bytes();
    // NOCOM(#sirver): do not unwrap, do error handling.
    str::from_utf8(buf).unwrap().into()
}

#[no_mangle]
pub extern "C" fn swiboe_connect(socket_name: *const c_char) -> *mut client::Client {
    let socket_name = c_str_to_string(socket_name);
    let socket_name_path = path::Path::new(&socket_name);

    let client = Box::new(
        // NOCOM(#sirver): error handling
        client::Client::connect(socket_name_path).unwrap(),
    );

    unsafe { mem::transmute(client) }
}

#[no_mangle]
pub extern "C" fn swiboe_disconnect(client: *mut client::Client) {
    unsafe {
        let _: Box<client::Client> = mem::transmute(client);
    }
}

struct CallbackRpc {
    priority: u16,
    callback: extern fn(*const c_char) -> libc::uint16_t,
}

const RPC_OK: libc::uint16_t = 0;
const RPC_ERR: libc::uint16_t = 1;
const RPC_NOT_HANDLED: libc::uint16_t = 2;

impl client::rpc::server::Rpc for CallbackRpc {
    fn priority(&self) -> u16 { self.priority }

    fn call(&self,
            mut context: client::rpc::server::Context,
            args: serde_json::Value) {
        let args_str = serde_json::to_string(&args).unwrap();
        let c_str = CString::new(args_str).expect("JSON contained zero byte");
        match (self.callback)(c_str.as_ptr()) {
            RPC_NOT_HANDLED => {
                context.finish(rpc::Result::NotHandled).unwrap();
            },
            RPC_OK => {
                // TODO(sirver): We would like to return something from our RPCs, not only an empty
                // value.
                context.finish(rpc::Result::success("")).unwrap();
            },
            RPC_ERR => { unimplemented!(); },
            _ => panic!("RPC callback returned invalid value."),
        }
    }
}

// NOCOM(#sirver): add error handling.
#[no_mangle]
pub extern "C" fn swiboe_new_rpc(client: *mut client::Client,
                                 rpc_name: *const c_char,
                                 priority: libc::uint16_t,
                                 callback: extern fn(*const c_char) -> libc::uint16_t
                                 ) {
    let client: &mut client::Client = unsafe {
        mem::transmute(client)
    };

    let rpc = Box::new(CallbackRpc {
        priority: priority,
        callback: callback,
    });

    let rpc_name= c_str_to_string(rpc_name);
    client.new_rpc(&rpc_name, rpc);
}
