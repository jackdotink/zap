mod api;
mod codegen;
mod ir;
mod types;

fn main() {
    let ty = match api::exec(std::fs::read("test").unwrap().as_slice()) {
        Ok(ty) => ty,
        Err(e) => {
            eprintln!("Error: {e}");
            return;
        }
    };

    println!("{}", ir::build(ty, ir::Check::Full));
}
