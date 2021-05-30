// Copyright 2021 Shin Yoshida
//
// "LGPL-3.0-or-later OR Apache-2.0 OR BSD-2-Clause"
//
// This is part of mouse-sqlite3
//
//  mouse-sqlite3 is free software: you can redistribute it and/or modify
//  it under the terms of the GNU Lesser General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  mouse-sqlite3 is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU Lesser General Public License for more details.
//
//  You should have received a copy of the GNU Lesser General Public License
//  along with mouse-sqlite3.  If not, see <http://www.gnu.org/licenses/>.
//
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//
// Redistribution and use in source and binary forms, with or without modification, are permitted
// provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of
//    conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright notice, this
//    list of conditions and the following disclaimer in the documentation and/or other
//    materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
// IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
// INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
// NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

use crate::Error;
use libsqlite3_sys::{
    sqlite3_clear_bindings, sqlite3_finalize, sqlite3_reset, sqlite3_step, sqlite3_stmt,
};
use std::os::raw::c_int;

/// Wrapper of C [`sqlite3_stmt`] .
///
/// [`sqlite3_stmt`]: https://www.sqlite.org/c3ref/stmt.html
pub struct Stmt {
    raw: *mut sqlite3_stmt,
    column_count: c_int,
    is_row: bool,
}

impl Drop for Stmt {
    fn drop(&mut self) {
        unsafe { sqlite3_finalize(self.raw) };
    }
}

impl Stmt {
    /// Calls C function [`sqlite3_reset`] to clear the previous result.
    ///
    /// This method is called automatically if necessary, so the user will rarely call this method.
    /// Note this method does not change the binding parameters at all.
    ///
    /// [`sqlite3_reset`]: https://www.sqlite.org/c3ref/reset.html
    pub fn reset(&mut self) {
        unsafe { sqlite3_reset(self.raw) };
        self.is_row = false;
    }

    /// Calls C function [`sqlite3_reset`] and [`sqlite3_clear_bindings`] to reset all the
    /// parameters.
    ///
    /// Because the document of [`sqlite3_clear_bindings`] is ambiguous, this method calls
    /// [`sqlite3_reset`] at the same time.
    ///
    /// # Panics
    ///
    /// Panics if [`sqlite3_clear_bindings`] failed.
    ///
    /// [`sqlite3_reset`]: https://www.sqlite.org/c3ref/reset.html
    /// [`sqlite3_clear_bindings`]: https://www.sqlite.org/c3ref/clear_bindings.html
    pub fn clear(&mut self) {
        self.reset();
        let code = unsafe { sqlite3_clear_bindings(self.raw) };
        let e = Error::new(code);
        if e != Error::OK {
            panic!("{}", e);
        }
    }

    /// Wrapper of C function [`sqlite3_step`] and returns whether the SQL statement returns any
    /// data to be fetched.
    ///
    /// Returns `true` if the SQL statement being executed returns any data (i.e. [`sqlite3_step`]
    /// returned `SQLITE_ROW`.)
    ///
    /// Calls [`reset`] and returns `false` if the SQL statement has finished (i.e.
    /// [`sqlite3_step`] returned `SQLITE_DONE` . Then no data was returned.)
    ///
    /// Otherwise, i.e. [`sqlite3_step`] failed, calls [`reset`] and returns `Err` .
    ///
    /// [`reset`]: #method.reset
    /// [`sqlite3_step`]: https://www.sqlite.org/c3ref/step.html
    pub fn step(&mut self) -> Result<bool, Error> {
        let code = unsafe { sqlite3_step(self.raw) };
        match Error::new(code) {
            Error::DONE => {
                self.reset();
                Ok(false)
            }
            Error::ROW => {
                self.is_row = true;
                Ok(true)
            }
            e => {
                self.reset();
                Err(e)
            }
        }
    }
}
