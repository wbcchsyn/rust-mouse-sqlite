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

use crate::{Error, Stmt};
use core::convert::TryFrom;
use core::hash::{Hash, Hasher};
use core::ptr::NonNull;
use libsqlite3_sys::{
    sqlite3, sqlite3_close, sqlite3_open_v2, sqlite3_prepare_v2, sqlite3_stmt, SQLITE_OPEN_CREATE,
    SQLITE_OPEN_NOMUTEX, SQLITE_OPEN_READWRITE, SQLITE_TOOBIG,
};
use std::collections::hash_map::{Entry, HashMap};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::Path;

/// New type of `&'static str` , which is compared by the address.
#[derive(Debug, Clone, Copy)]
struct Sql(*const u8);

impl PartialEq for Sql {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0, other.0)
    }
}

impl Eq for Sql {}

impl Hash for Sql {
    #[inline]
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        (self.0 as usize).hash(hasher)
    }
}

/// Wrapper of C [`sqlite3 *`] with cache of [`Stmt`] .
///
/// [`sqlite3 *`]: https://www.sqlite.org/c3ref/sqlite3.html
/// [`Stmt`]: struct.Stmt.html
pub struct Connection {
    raw: *mut sqlite3,
    stmts: HashMap<Sql, Stmt>,
}

impl Drop for Connection {
    #[inline]
    fn drop(&mut self) {
        self.stmts.clear(); // All the Stmt instances must be finalized before close.
        unsafe { sqlite3_close(self.raw) };
    }
}

impl TryFrom<&Path> for Connection {
    type Error = Box<dyn std::error::Error>;

    #[inline]
    fn try_from(filename: &Path) -> Result<Self, Self::Error> {
        let filename = CString::new(filename.to_string_lossy().as_bytes()).map_err(Box::new)?;
        let mut raw: *mut sqlite3 = core::ptr::null_mut();
        const FLAGS: c_int = SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_NOMUTEX;
        const ZVFS: *const c_char = core::ptr::null();

        let code = unsafe { sqlite3_open_v2(filename.as_ptr(), &mut raw, FLAGS, ZVFS) };
        match Error::new(code) {
            Error::OK => Ok(Self {
                raw,
                stmts: Default::default(),
            }),
            e => Err(Box::new(e)),
        }
    }
}

impl Connection {
    /// Creates and caches [`Stmt`] if not cached and provides a reference to the cached instance.
    ///
    /// [`Stmt`]: struct.Stmt.html
    #[inline]
    pub fn stmt(&mut self, sql: &'static str) -> Result<&mut Stmt, Error> {
        match self.stmts.entry(Sql(sql.as_ptr())) {
            Entry::Occupied(o) => {
                let stmt = o.into_mut();
                stmt.clear();
                Ok(stmt)
            }
            Entry::Vacant(v) => {
                let stmt = Self::build_stmt(self.raw, sql)?;
                Ok(v.insert(stmt))
            }
        }
    }

    /// Creates [`Stmt`] instance.
    ///
    /// [`Stmt`]: struct.Stmt.html
    #[inline]
    pub fn stmt_once(&mut self, sql: &str) -> Result<Stmt, Error> {
        Self::build_stmt(self.raw, sql)
    }

    #[inline]
    fn build_stmt(raw: *mut sqlite3, sql: &str) -> Result<Stmt, Error> {
        let zsql = sql.as_ptr() as *const c_char;
        let nbytes = c_int::try_from(sql.len()).map_err(|_| Error::new(SQLITE_TOOBIG))?;
        let mut raw_stmt: *mut sqlite3_stmt = core::ptr::null_mut();
        let mut pztail: *const c_char = core::ptr::null();

        let code = unsafe { sqlite3_prepare_v2(raw, zsql, nbytes, &mut raw_stmt, &mut pztail) };
        match Error::new(code) {
            Error::OK => {
                let ptr = NonNull::new(raw_stmt).unwrap();
                Ok(crate::stmt_from_raw(ptr))
            }
            e => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;
    use core::convert::TryFrom;
    use tempfile::tempdir;

    #[test]
    fn create() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().to_owned();
        let path = path.join("test_sqlite");
        assert!(Connection::try_from(path.as_ref()).is_ok());
    }

    #[test]
    fn create_table() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().to_owned();
        let path = path.join("test_sqlite");
        let mut con = Connection::try_from(path.as_ref()).unwrap();

        let sql = r#"CREATE TABLE IF NOT EXISTS "foo" (
            "_id" INTEGER PRIMARY KEY,
            "value" TEXT
        )"#;
        let mut stmt = con.stmt_once(sql).unwrap();
        assert_eq!(Ok(false), stmt.step());
    }
}
