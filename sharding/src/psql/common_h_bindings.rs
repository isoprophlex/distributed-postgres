/* automatically generated by rust-bindgen 0.69.4 */

#[repr(C)]
#[derive(Default)]
pub struct __IncompleteArrayField<T>(::core::marker::PhantomData<T>, [T; 0]);
impl<T> __IncompleteArrayField<T> {
    #[inline]
    pub const fn new() -> Self {
        __IncompleteArrayField(::core::marker::PhantomData, [])
    }
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self as *const _ as *const T
    }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut _ as *mut T
    }
    #[inline]
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        ::core::slice::from_raw_parts(self.as_ptr(), len)
    }
    #[inline]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        ::core::slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}
impl<T> ::core::fmt::Debug for __IncompleteArrayField<T> {
    fn fmt(&self, fmt: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        fmt.write_str("__IncompleteArrayField")
    }
}
pub type Oid = libc::c_uint;
pub type pg_int64 = libc::c_long;
pub type __uint16_t = libc::c_ushort;
pub type __int32_t = libc::c_int;
pub type __uint32_t = libc::c_uint;
pub type __uint64_t = libc::c_ulong;
pub type __uid_t = libc::c_uint;
pub type __gid_t = libc::c_uint;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub type __pid_t = libc::c_int;
pub type __clock_t = libc::c_long;
pub type __time_t = libc::c_long;
pub type __suseconds_t = libc::c_long;
pub type __clockid_t = libc::c_int;
pub type __timer_t = *mut libc::c_void;
pub type __ssize_t = libc::c_long;
pub type __syscall_slong_t = libc::c_long;
pub type __sig_atomic_t = libc::c_int;
pub type PGEventId = libc::c_uint;
pub type FILE = _IO_FILE;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _IO_marker {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _IO_codecvt {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _IO_wide_data {
    _unused: [u8; 0],
}
pub type _IO_lock_t = libc::c_void;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1usize],
    pub _lock: *mut _IO_lock_t,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut libc::c_void,
    pub __pad5: usize,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20usize],
}
extern "C" {
    pub static mut stdin: *mut FILE;
}
extern "C" {
    pub static mut stdout: *mut FILE;
}
extern "C" {
    pub static mut stderr: *mut FILE;
}
pub type ExecStatusType = libc::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pgresAttValue {
    pub len: libc::c_int,
    pub value: *mut libc::c_char,
}
pub type PGresAttValue = pgresAttValue;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pgresParamDesc {
    pub typid: Oid,
}
pub type PGresParamDesc = pgresParamDesc;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PGNoticeHooks {
    pub noticeRec: PQnoticeReceiver,
    pub noticeRecArg: *mut libc::c_void,
    pub noticeProc: PQnoticeProcessor,
    pub noticeProcArg: *mut libc::c_void,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PGEvent {
    pub proc_: PGEventProc,
    pub name: *mut libc::c_char,
    pub passThrough: *mut libc::c_void,
    pub data: *mut libc::c_void,
    pub resultInitialized: bool,
}
pub type PGEventProc = ::core::option::Option<
    unsafe extern "C" fn(
        evtId: PGEventId,
        evtInfo: *mut libc::c_void,
        passThrough: *mut libc::c_void,
    ) -> libc::c_int,
>;
#[repr(C)]
#[derive(Debug)]
pub struct pgMessageField {
    pub next: *mut pgMessageField,
    pub code: libc::c_char,
    pub contents: __IncompleteArrayField<libc::c_char>,
}
pub type PGMessageField = pgMessageField;
pub type PGresult_data = pgresult_data;
#[repr(C)]
#[derive(Copy, Clone)]
pub union pgresult_data {
    pub next: *mut PGresult_data,
    pub space: [libc::c_char; 1usize],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pg_result {
    pub ntups: libc::c_int,
    pub numAttributes: libc::c_int,
    pub attDescs: *mut PGresAttDesc,
    pub tuples: *mut *mut PGresAttValue,
    pub tupArrSize: libc::c_int,
    pub numParameters: libc::c_int,
    pub paramDescs: *mut PGresParamDesc,
    pub resultStatus: ExecStatusType,
    pub cmdStatus: [libc::c_char; 64usize],
    pub binary: libc::c_int,
    pub noticeHooks: PGNoticeHooks,
    pub events: *mut PGEvent,
    pub nEvents: libc::c_int,
    pub client_encoding: libc::c_int,
    pub errMsg: *mut libc::c_char,
    pub errFields: *mut PGMessageField,
    pub errQuery: *mut libc::c_char,
    pub null_field: [libc::c_char; 1usize],
    pub curBlock: *mut PGresult_data,
    pub curOffset: libc::c_int,
    pub spaceLeft: libc::c_int,
    pub memorySize: usize,
}
pub type PGresult = pg_result;
pub type PQnoticeReceiver =
    ::core::option::Option<unsafe extern "C" fn(arg: *mut libc::c_void, res: *const PGresult)>;
pub type PQnoticeProcessor = ::core::option::Option<
    unsafe extern "C" fn(arg: *mut libc::c_void, message: *const libc::c_char),
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pgresAttDesc {
    pub name: *mut libc::c_char,
    pub tableid: Oid,
    pub columnid: libc::c_int,
    pub format: libc::c_int,
    pub typid: Oid,
    pub typlen: libc::c_int,
    pub atttypmod: libc::c_int,
}
pub type PGresAttDesc = pgresAttDesc;
pub type printFormat = libc::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct printTextLineFormat {
    pub hrule: *const libc::c_char,
    pub leftvrule: *const libc::c_char,
    pub midvrule: *const libc::c_char,
    pub rightvrule: *const libc::c_char,
}
pub type printXheaderWidthType = libc::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct printTextFormat {
    pub name: *const libc::c_char,
    pub lrule: [printTextLineFormat; 4usize],
    pub midvrule_nl: *const libc::c_char,
    pub midvrule_wrap: *const libc::c_char,
    pub midvrule_blank: *const libc::c_char,
    pub header_nl_left: *const libc::c_char,
    pub header_nl_right: *const libc::c_char,
    pub nl_left: *const libc::c_char,
    pub nl_right: *const libc::c_char,
    pub wrap_left: *const libc::c_char,
    pub wrap_right: *const libc::c_char,
    pub wrap_right_border: bool,
}
pub type unicode_linestyle = libc::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct separator {
    pub separator: *mut libc::c_char,
    pub separator_zero: bool,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct printTableOpt {
    pub format: printFormat,
    pub expanded: libc::c_ushort,
    pub expanded_header_width_type: printXheaderWidthType,
    pub expanded_header_exact_width: libc::c_int,
    pub border: libc::c_ushort,
    pub pager: libc::c_ushort,
    pub pager_min_lines: libc::c_int,
    pub tuples_only: bool,
    pub start_table: bool,
    pub stop_table: bool,
    pub default_footer: bool,
    pub prior_records: libc::c_ulong,
    pub line_style: *const printTextFormat,
    pub fieldSep: separator,
    pub recordSep: separator,
    pub csvFieldSep: [libc::c_char; 2usize],
    pub numericLocale: bool,
    pub tableAttr: *mut libc::c_char,
    pub encoding: libc::c_int,
    pub env_columns: libc::c_int,
    pub columns: libc::c_int,
    pub unicode_border_linestyle: unicode_linestyle,
    pub unicode_column_linestyle: unicode_linestyle,
    pub unicode_header_linestyle: unicode_linestyle,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct printTableFooter {
    pub data: *mut libc::c_char,
    pub next: *mut printTableFooter,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct printTableContent {
    pub opt: *const printTableOpt,
    pub title: *const libc::c_char,
    pub ncolumns: libc::c_int,
    pub nrows: libc::c_int,
    pub headers: *mut *const libc::c_char,
    pub header: *mut *const libc::c_char,
    pub cells: *mut *const libc::c_char,
    pub cell: *mut *const libc::c_char,
    pub cellsadded: u64,
    pub cellmustfree: *mut bool,
    pub footers: *mut printTableFooter,
    pub footer: *mut printTableFooter,
    pub aligns: *mut libc::c_char,
    pub align: *mut libc::c_char,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct printQueryOpt {
    pub topt: printTableOpt,
    pub nullPrint: *mut libc::c_char,
    pub title: *mut libc::c_char,
    pub footers: *mut *mut libc::c_char,
    pub translate_header: bool,
    pub translate_columns: *const bool,
    pub n_translate_columns: libc::c_int,
}
extern "C" {
    pub fn printTable(
        cont: *const printTableContent,
        fout: *mut FILE,
        is_pager: bool,
        flog: *mut FILE,
    );
}
extern "C" {
    pub fn printQuery(
        result: *const PGresult,
        opt: *const printQueryOpt,
        fout: *mut FILE,
        is_pager: bool,
        flog: *mut FILE,
    );
}
extern "C" {
    pub fn PSQLexec(query: *const libc::c_char) -> *mut PGresult;
}
extern "C" {
    pub fn SendQuery(query: *const libc::c_char) -> bool;
}
extern "C" {
    pub fn is_superuser() -> bool;
}
