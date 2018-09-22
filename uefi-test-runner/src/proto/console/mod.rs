use uefi::table::SystemTable;

pub fn test(st: &SystemTable) {
    stdout::test(st.stdout());

    let bt = st.boot;
    serial::test(bt);
    gop::test(bt);
    pointer::test(bt);
}

mod gop;
mod pointer;
mod serial;
mod stdout;
