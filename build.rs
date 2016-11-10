extern crate svd_parser;
extern crate svd_codegen;
extern crate inflections;

use std::fs::{self, File};
use std::io::{self, Read, Write};
use inflections::Inflect;

fn main() {
    codegen(svd_file_name()).expect("codegen failed");

    println!("cargo:rerun-if-changed=svd/STM32F7x.svd");
}

fn svd_file_name() -> &'static str {
    use std::env;

    let err_no_board = "Please choose a board feature!";
    let err_duplicate_board = "Error: Multiple board features chosen!";

    let mut file_name = None;

    if env::var("CARGO_FEATURE_STM32F7").is_ok() {
        assert!(file_name.is_none(), err_duplicate_board);
        file_name = Some("svd/STM32F7x.svd");
    }

    file_name.expect(err_no_board)
}

fn codegen(svd_file_name: &str) -> io::Result<()> {
    let xml = &mut String::new();
    File::open(svd_file_name).unwrap().read_to_string(xml).unwrap();
    let device = svd_parser::parse(xml);

    let _ = fs::remove_dir_all("src");
    fs::create_dir("src").unwrap();

    let mut lib_rs = File::create("src/lib.rs").unwrap();

    // no_std
    try!(writeln!(lib_rs, "#![no_std]"));
    try!(writeln!(lib_rs, ""));

    // extern crates
    try!(writeln!(lib_rs, "extern crate volatile;"));
    try!(writeln!(lib_rs, "#[macro_use] extern crate once;"));
    try!(writeln!(lib_rs, ""));

    let mut modules = Vec::new();

    for peripheral in &device.peripherals {
        let code = svd_codegen::gen_peripheral(peripheral, &device.defaults)
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n\n");

        if code.len() > 0 {
            let name = peripheral.name.to_lowercase();
            println!("{}", name);
            modules.push((name.clone(), peripheral.base_address));
            try!(writeln!(lib_rs, "pub mod {};", name));

            let file_name = format!("src/{}.rs", name);
            let mut file = File::create(file_name).unwrap();

            try!(write!(file, "// autogenerated, do not edit\n\n{}", code));
        }
    }

    // hardware struct
    try!(writeln!(lib_rs, ""));
    try!(writeln!(lib_rs, "pub struct Hardware {{"));
    for &(ref module_name, _) in &modules {
        try!(writeln!(lib_rs,
                      "    pub {}: &'static mut {}::{},",
                      module_name,
                      module_name,
                      module_name.to_pascal_case()));
    }
    try!(writeln!(lib_rs, "}}"));

    // hw function
    try!(writeln!(lib_rs, ""));
    try!(writeln!(lib_rs, "pub fn hw() -> Hardware {{"));
    try!(writeln!(lib_rs, "    assert_has_not_been_called!();"));
    try!(writeln!(lib_rs, "    Hardware {{"));
    for &(ref module_name, ref base_address) in &modules {
        try!(writeln!(lib_rs,
                      "        {}: unsafe {{ from_addr({:#x}) }},",
                      module_name,
                      base_address));
    }
    try!(writeln!(lib_rs, "    }}"));
    try!(writeln!(lib_rs, "}}"));
    try!(writeln!(lib_rs,
                  "\nunsafe fn from_addr<T>(addr: usize) -> &'static mut T {{"));
    try!(writeln!(lib_rs, "    &mut *(addr as *const T as *mut T)"));
    try!(writeln!(lib_rs, "}}"));

    Ok(())
}
