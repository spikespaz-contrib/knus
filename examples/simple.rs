use std::io::Read;

use miette::IntoDiagnostic;


#[derive(knus::Decode, Debug)]
#[allow(dead_code)]
struct Plugin {
    #[knus(argument)]
    name: String,
    #[knus(property)]
    url: String,
    #[knus(child, unwrap(argument))]
    version: String,
}

#[derive(knus::Decode, Debug)]
#[allow(dead_code)]
struct Config {
    #[knus(child, unwrap(argument))]
    version: String,
    #[knus(children(name="plugin"))]
    plugins: Vec<Plugin>,
}

fn main() -> miette::Result<()> {
    let mut buf = String::new();
    println!("Please type KDL document, press Return, Ctrl+D to finish");
    std::io::stdin().read_to_string(&mut buf).into_diagnostic()?;
    let cfg: Config = knus::parse("<stdin>", &buf)?;
    println!("{:#?}", cfg);
    Ok(())
}
