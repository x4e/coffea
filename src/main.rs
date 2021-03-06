use std::{
    fs::File,
    io::{stdout, BufReader},
};

use coffea::{ClassFile, JResult};

fn main() -> JResult<()> {
    let out = std::process::Command::new("javac")
        .args(&["test.java"])
        .output()
        .unwrap();
    if !out.stderr.is_empty() {
        dbg!(out);
        std::process::exit(1);
    }
    let reader = BufReader::new(File::open("test.class")?);
    let file = ClassFile::from_bufreader(reader)?;
    // let mut outfile = File::create("testout.java")?;
    let outfile = stdout();

    file.print(outfile)?;

    // dbg!(&file.method_by_name("main"));
    // for name in file.method_names() {
    //     dbg!(&file.method_by_name(name).unwrap().code().unwrap().lex());
    //     file.clone().codegen(name, &mut outfile)?;
    // }
    // dbg!(file.method_names());

    // dbg!(file.method_by_name("main").unwrap().code().unwrap().lex());
    // file.codegen("main", &mut outfile)?;
    Ok(())
}
