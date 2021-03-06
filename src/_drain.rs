/// Logging drain
///
/// `Drain`s typially mean destination for logs, but `slog` generalizes the
/// term.
///
/// `Drain`s are responsible for handling logging statements (`Record`s) from
/// `Logger`s associated with them: filtering, modifying, formatting
/// and writing the log records into given destination(s).
///
/// Implementing this trait allows writing custom `Drain`s.
///
/// It's a typical pattern to parametrize `Drain`s over `Drain` traits to allow
/// combining them into one `Drain`. See `Duplicate` as an example.
pub trait Drain {
    /// Type returned by this drain
    ///
    /// It can be useful in some circumstances, but rarely. It will probably default to `()` once
    /// https://github.com/rust-lang/rust/issues/29661 is stable.
    type Ok;
    /// Type of potential errors that can be returned by this `Drain`
    type Err;
    /// Handle one logging statement (`Record`)
    ///
    /// Every logging `Record` built from a logging statement (eg.
    /// `info!(...)`), and key-value lists of a `Logger` it was executed on
    /// will be passed to the root drain registered during `Logger::root`.
    ///
    /// Typically `Drain`s:
    ///
    /// * pass this information (or not) to the sub-logger(s) (filters)
    /// * format and write the information the a destination (writers)
    /// * deal with the errors returned from the sub-logger(s)
    fn log(&self, record: &Record, values : &OwnedKVList) -> result::Result<Self::Ok, Self::Err>;
}

impl<D: Drain+?Sized> Drain for Box<D> {
    type Ok = D::Ok;
    type Err = D::Err;
    fn log(&self, record: &Record, o: &OwnedKVList) -> result::Result<Self::Ok, D::Err> {
        (**self).log(record, o)
    }
}

impl<D: Drain+?Sized> Drain for Arc<D> {
    type Ok = D::Ok;
    type Err = D::Err;
    fn log(&self, record: &Record, o: &OwnedKVList) -> result::Result<Self::Ok, D::Err> {
        (**self).log(record, o)
    }
}

/// Convenience methods for building drains
///
/// `DrainExt` is implemented for every `Drain` and contains
/// convenience methods.
pub trait DrainExt: Sized + Drain {
    /// Map logging errors returned by this drain
    ///
    /// `f` is a closure that takes `Drain::Err` returned by a given
    /// drain, and returns new error of potentially different type
    fn map_err<F, E>(self, f : F) -> MapError<Self, E> where F : 'static + Sync + Send + Fn(<Self as Drain>::Err) -> E {
        MapError::new(self, f)
    }

    /// Make `Self` ignore and result
    fn ignore_res(self) -> IgnoreResult<Self> {
        IgnoreResult::new(self)
    }

    /// Make `Self` panic when returning any errors
    fn fuse(self) -> Fuse<Self> where <Self as Drain>::Err : fmt::Display {
       Fuse::new(self)
    }
}

impl<D : Drain> DrainExt for D {}

/// `Drain` discarding everything
///
/// `/dev/null` of `Drain`s
pub struct Discard;

impl Drain for Discard {
    type Ok = ();
    type Err = Never;
    fn log(&self, _: &Record, _: &OwnedKVList) -> result::Result<(), Never> {
        Ok(())
    }
}

/// `Drain` filtering records
///
/// Wraps another `Drain` and passes `Record`s to it, only if they satisfy a
/// given condition.
pub struct Filter<D: Drain> {
    drain: D,
    // eliminated dynamic dispatch, after rust learns `-> impl Trait`
    cond: Box<Fn(&Record) -> bool + 'static + Send + Sync>,
}

impl<D: Drain> Filter<D> {
    /// Create `Filter` wrapping given `drain`
    pub fn new<F: 'static + Sync + Send + Fn(&Record) -> bool>(drain: D, cond: F) -> Self {
        Filter {
            drain: drain,
            cond: Box::new(cond),
        }
    }
}

impl<D: Drain> Drain for Filter<D> {
    type Ok = Option<D::Ok>;
    type Err = D::Err;
    fn log(&self,
           record: &Record,
           logger_values: &OwnedKVList)
           -> result::Result<Self::Ok, Self::Err> {
        if (self.cond)(&record) {
            Ok(Some(self.drain.log(record, logger_values)?))
        } else {
            Ok(None)
        }
    }
}


/// `Drain` mapping error returned by another `Drain`
///
/// See `DrainExt::map_err` for convenience function.
pub struct MapError<D: Drain, E> {
    drain: D,
    // eliminated dynamic dispatch, after rust learns `-> impl Trait`
    map_fn: Box<Fn(D::Err) -> E + 'static+ Send+Sync>,
}

impl<D: Drain, E> MapError<D, E> {
    /// Create `Filter` wrapping given `drain`
    pub fn new<F: 'static + Sync + Send + Fn(<D as Drain>::Err) -> E>(drain: D, map_fn: F) -> Self {
        MapError {
            drain: drain,
            map_fn: Box::new(map_fn),
        }
    }
}

impl<D: Drain, E> Drain for MapError<D, E> {
    type Ok = D::Ok;
    type Err = E;
    fn log(&self,
           record: &Record,
           logger_values: &OwnedKVList)
           -> result::Result<Self::Ok, Self::Err> {
            self.drain.log(record, logger_values).map_err(|e| (self.map_fn)(e))
    }
}


/// `Drain` filtering records by `Record` logging level
///
/// Wraps a drain and passes records to it, only
/// if their level is at least given level.
///
/// TODO: Remove this type. This drain is a special case of `Filter`, but
/// because `Filter` can not use static dispatch ATM due to Rust limitations
/// that will be lifted in the future, it is a standalone type.
/// Reference: https://github.com/rust-lang/rust/issues/34511
pub struct LevelFilter<D: Drain> {
    level: Level,
    drain: D,
}

impl<D: Drain> LevelFilter<D> {
    /// Create `LevelFilter`
    pub fn new(drain: D, level: Level) -> Self {
        LevelFilter {
            level: level,
            drain: drain,
        }
    }
}

impl<D: Drain> Drain for LevelFilter<D> {
    type Ok = Option<D::Ok>;
    type Err = D::Err;
    fn log(&self,
           record: &Record,
           logger_values: &OwnedKVList)
           -> result::Result<Self::Ok, Self::Err> {
        if record.level().is_at_least(self.level) {
            Ok(Some(self.drain.log(record, logger_values)?))
        } else {
            Ok(None)
        }
    }
}

/// `Drain` duplicating records into two other `Drain`s
///
/// Can be nested for more than two outputs.
pub struct Duplicate<D1: Drain, D2: Drain> {
    drain1: D1,
    drain2: D2,
}


impl<D1: Drain, D2: Drain> Duplicate<D1, D2> {
    /// Create `Duplicate`
    pub fn new(drain1: D1, drain2: D2) -> Self {
        Duplicate {
            drain1: drain1,
            drain2: drain2,
        }
    }
}

impl<D1 : Drain, D2 : Drain> Drain for Duplicate<D1, D2> {
    type Ok = (D1::Ok, D2::Ok);
    type Err = (result::Result<D1::Ok, D1::Err>, result::Result<D2::Ok, D2::Err>);
    fn log(&self,
           record: &Record,
           logger_values: &OwnedKVList)
           -> result::Result<Self::Ok, Self::Err> {
        let res1 = self.drain1.log(record, logger_values);
        let res2 = self.drain2.log(record, logger_values);

        match (res1, res2) {
            (Ok(o1), Ok(o2)) => Ok((o1, o2)),
            (r1, r2) => Err((r1, r2)),
        }
    }
}


/// `Drain` panicking on error
///
/// `Logger` requires a root drain to handle all errors (`Drain::Error == ()`),
/// `Fuse` will wrap a `Drain` and panic if it returns any errors.
///
/// Note: `Drain::Err` must implement `Display` (for displaying on panick). It's
/// easy to create own `Fuse` drain if this requirement can't be fulfilled.
pub struct Fuse<D: Drain> {
    drain: D,
}

impl<D: Drain> Fuse<D> {
    /// Create `Fuse` wrapping given `drain`
    pub fn new(drain: D) -> Self {
        Fuse {
            drain: drain,
        }
    }
}

impl<D: Drain> Drain for Fuse<D> where D::Err : fmt::Display {
    type Ok = ();
    type Err = Never;
    fn log(&self,
           record: &Record,
           logger_values: &OwnedKVList)
        -> result::Result<Self::Ok, Never> {
            let _ = self.drain.log(record, logger_values).unwrap_or_else(
                |e| panic!("slog::Fuse Drain: {}", e)
                );
            Ok(())
        }
}


/// `Drain` ignoring result
///
/// `Logger` requires a root drain to handle all errors (`Drain::Err=()`), and
/// returns nothing (`Drain::Ok=()`) `IgnoreResult` will ignore any result
/// returned by the `Drain` it wraps.
pub struct IgnoreResult<D: Drain> {
    drain: D,
}

impl<D: Drain> IgnoreResult<D> {
    /// Create `IgnoreResult` wrapping `drain`
    pub fn new(drain: D) -> Self {
        IgnoreResult {
            drain: drain,
        }
    }
}

impl<D: Drain> Drain for IgnoreResult<D> {
    type Ok = ();
    type Err = Never;
    fn log(&self,
           record: &Record,
           logger_values: &OwnedKVList)
        -> result::Result<(), Never> {
            let _ = self.drain.log(record, logger_values);
            Ok(())
        }
}

/// Filter by `cond` closure
pub fn filter<D: Drain, F: 'static + Send + Sync + Fn(&Record) -> bool>(
    cond: F,
    d: D
    ) -> Filter<D> {
    Filter::new(d, cond)
}

/// Filter by log level
pub fn level_filter<D: Drain>(level: Level, d: D) -> LevelFilter<D> {
    LevelFilter::new(d, level)
}

/// Duplicate records to two drains
///
/// Create `Duplicate` drain.
///
/// Can be nested for multiple outputs.
pub fn duplicate<D1: Drain, D2: Drain>(d1: D1, d2: D2) -> Duplicate<D1, D2> {
    Duplicate::new(d1, d2)
}

#[cfg(feature = "std")]
#[derive(Debug)]
/// Error returned by `Mutex<D : Drain>`
pub enum MutexDrainError<D : Drain> {
    /// Error aquiring mutex
    Mutex,
    /// Error returned by drain
    Drain(D::Err),
}

#[cfg(feature = "std")]
impl<'a, D : Drain> From<std::sync::PoisonError<std::sync::MutexGuard<'a, D>>> for MutexDrainError<D> {
    fn from(_ : std::sync::PoisonError<std::sync::MutexGuard<'a, D>>) -> MutexDrainError<D> {
        MutexDrainError::Mutex
    }
}
#[cfg(feature = "std")]
impl<D : Drain> fmt::Display for MutexDrainError<D>
where D::Err : fmt::Display {
    fn fmt(&self, f : &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        match *self {
            MutexDrainError::Mutex => write!(f, "MutexError"),
            MutexDrainError::Drain(ref e) => write!(f, "{}", e),
        }
    }
}

#[cfg(feature = "std")]
impl<D: Drain> Drain for std::sync::Mutex<D> {
    type Ok = D::Ok;
    type Err = MutexDrainError<D>;
    fn log(&self,
           record: &Record,
           logger_values: &OwnedKVList)
        -> result::Result<Self::Ok, Self::Err> {
            let d = self.lock()?;
            d.log(record, logger_values).map_err(MutexDrainError::Drain)
        }
}
