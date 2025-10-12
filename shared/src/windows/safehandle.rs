// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//    * Redistributions of source code must retain the above copyright notice,
//      this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above copyright notice,
//      this list of conditions and the following disclaimer in the documentation
//      and/or other materials provided with the distribution.
//    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
//      may be used to endorse or promote products derived from this software
//      without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
/*!
Author: Adolfo Gómez, dkmaster at dkmon dot com
*/
use std::sync::Arc;
use windows::{
    Win32::Foundation::{CloseHandle, HANDLE},
};

#[derive(Debug)]
struct HandleInner {
    handle: HANDLE,
    owned: bool,
}

unsafe impl Send for HandleInner {}
unsafe impl Sync for HandleInner {}

#[derive(Debug, Clone)]
pub struct SafeHandle {
    inner: Arc<HandleInner>,
}

#[allow(dead_code)]
impl SafeHandle {
    /// Creates a SafeHandle that owns the handle (will close it on Drop)
    pub fn new(handle: HANDLE) -> Self {
        Self {
            inner: Arc::new(HandleInner {
                handle,
                owned: true,
            }),
        }
    }

    /// Creates a SafeHandle that does NOT own the handle (will NOT close it on Drop)
    pub fn from_borrowed(handle: HANDLE) -> Self {
        Self {
            inner: Arc::new(HandleInner {
                handle,
                owned: false,
            }),
        }
    }

    pub fn get(&self) -> HANDLE {
        self.inner.handle
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.handle.is_invalid()
    }

    pub fn into_raw(self) -> *mut core::ffi::c_void {
        let handle = self.get();
        std::mem::forget(self);
        handle.0
    }

    /// Creates a non-owning SafeHandle from a raw HANDLE pointer
    pub fn from_raw(handle: *mut core::ffi::c_void) -> Self {
        let handle = HANDLE(handle);
        Self::from_borrowed(handle)
    }
}

impl Drop for HandleInner {
    fn drop(&mut self) {
        if self.owned && !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

// Implement only From for HANDLE -> SafeHandle (owned)
impl From<HANDLE> for SafeHandle {
    fn from(handle: HANDLE) -> Self {
        SafeHandle::new(handle)
    }
}

// Dereference to HANDLE
impl AsRef<HANDLE> for SafeHandle {
    fn as_ref(&self) -> &HANDLE {
        &self.inner.handle
    }
}

impl std::fmt::Display for SafeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SafeHandle({:p}, {})", self.inner.handle.0, self.inner.owned)
    }
}

impl Default for SafeHandle {
    fn default() -> Self {
        Self::new(HANDLE::default()) // Invalid handle, owned but does nothing on drop
    }
}
