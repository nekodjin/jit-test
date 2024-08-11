use std::io::Read as _;
use std::io::Write as _;
use std::process::Command;

fn main() {
    let rust_msg = "This is a rust string!\n";

    let content = format!(
        r#"
        bits 64
        default rel

        section .text

        mov rdx, msg_len
        lea rsi, [msg_ptr]
        mov rdi, 1
        mov rax, 1
        syscall

        mov rdx, {rust_len}
        mov rsi, {rust_ptr}
        mov rdi, 1
        mov rax, 1
        syscall

        ret

        section .data

        msg_ptr db "Hello, world!", 0x0a
        msg_len equ $ - msg_ptr
        "#,
        rust_ptr = rust_msg.as_ptr() as usize,
        rust_len = rust_msg.len(),
    );

    let mut in_file = tempfile::NamedTempFile::new().unwrap();
    let mut out_file = tempfile::NamedTempFile::new().unwrap();

    write!(in_file, "{}", content).unwrap();
    in_file.flush().unwrap();

    let nasm_status = Command::new("nasm")
        .arg("-fbin")
        .arg("-o")
        .arg(out_file.path())
        .arg(in_file.path())
        .status()
        .unwrap();

    if !nasm_status.success() {
        panic!("nasm failed");
    }

    drop(in_file);

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };

    let xpage = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            page_size.try_into().unwrap(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    let mut code = Vec::new();
    out_file.read_to_end(&mut code).unwrap();

    drop(out_file);

    {
        let xpage_slice = unsafe { std::slice::from_raw_parts_mut(xpage as *mut u8, code.len()) };

        xpage_slice.copy_from_slice(&code);
    }

    unsafe {
        libc::mprotect(
            xpage,
            page_size.try_into().unwrap(),
            libc::PROT_READ | libc::PROT_EXEC,
        )
    };

    let func: fn() = unsafe { std::mem::transmute(xpage) };

    func();
}
