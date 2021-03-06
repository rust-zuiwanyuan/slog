#[doc(hidden)]
/// Information that can be static in the given record thus allowing to optimize record creation to
/// be done mostly at compile-time.
///
/// This is not cosidered a part of stable API, and macros should be used instead.
pub struct RecordStatic<'a> {
    /// Logging level
    pub level: Level,
    /// File
    pub file: &'static str,
    /// Line
    pub line: u32,
    /// Column (currently not implemented)
    pub column: u32,
    /// Function (currently not implemented)
    pub function: &'static str,
    /// Module
    pub module: &'static str,
    /// Target - for backward compatibility with `log`
    pub target: &'a str,
}

/// One logging record
///
/// Corresponds to one logging statement like `info!(...)` and carries all it's
/// data: eg. message, immediate key-value pairs and key-value pairs of `Logger` used to execute it.
///
/// Record is passed to a `Logger`, which delivers it to it's own `Drain`,
/// where actual logging processing is implemented.
pub struct Record<'a> {
    meta: &'a RecordStatic<'a>,
    msg: fmt::Arguments<'a>,
    kv: BorrowedKV<'a>,
}


impl<'a> Record<'a> {
    /// Create a new `Record`
    ///
    /// This function is not considered a part of stable API
    #[inline]
    #[doc(hidden)]
    pub fn new(
        s : &'a RecordStatic<'a>,
        msg: fmt::Arguments<'a>,
        kv: BorrowedKV<'a>,
        ) -> Self {
        Record {
            meta: s,
            msg: msg,
            kv: kv,
        }
    }

    /// Get a log record message
    pub fn msg(&self) -> fmt::Arguments {
        self.msg
    }

    /// Get record logging level
    pub fn level(&self) -> Level {
        self.meta.level
    }

    /// Get line number
    pub fn line(&self) -> u32 {
        self.meta.line
    }

    /// Get error column
    pub fn column(&self) -> u32 {
        self.meta.column
    }

    /// Get file path
    pub fn file(&self) -> &'static str {
        self.meta.file
    }

    /// Get target
    ///
    /// Mostly for backward compatibility with `log`
    pub fn target(&self) -> &str {
        self.meta.target
    }

    /// Get module
    pub fn module(&self) -> &'static str {
        self.meta.module
    }

    /// Get function
    pub fn function(&self) -> &'static str {
        self.meta.function
    }

    /// Get key-value pairs
    pub fn kv(&self) -> BorrowedKV {
        BorrowedKV(self.kv.0)
    }
}
